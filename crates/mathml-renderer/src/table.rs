use alloc::string::String;
use core::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::ast::{Indentation, Node};
use crate::fmt::new_line_and_indent;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ColumnAlignment {
    LeftJustified = 1,
    Centered,
    RightJustified,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LineType {
    Solid = 3,
    Dashed = 4,
}

/// A column spec is the result of parsing a column specifier, like `{|c|l|}` for an array.
/// Each entry can either be a column with content (with an alignment and an optional line to the
/// right), or a column that is just a line (with no content).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ColumnSpecEntry {
    WithContent {
        alignment: ColumnAlignment,
        border_right: Option<LineType>,
    },
    OnlyLine(LineType),
}

pub type ColumnSpec<'arena> = &'arena [ColumnSpecEntry];

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ArraySpec<'arena> {
    /// This field determines whether we need to draw a line to the left of the first column. If
    /// `None`, no line is drawn. If `Some(LineType)`, a line of that type is drawn.
    pub border_left: Option<LineType>,
    /// `true` if this is a subarray (i.e., a `subarray` environment in LaTeX). Subarrays have
    /// different padding rules.
    pub is_sub: bool,
    pub column_spec: ColumnSpec<'arena>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Alignment {
    Centered,
    Cases,
    Alternating,
}

/// Equation-number / label metadata for a row of a numbered environment
/// (`align`, `gather`, `equation`, `multline`, etc.).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RowLabelInfo<'arena> {
    pub tag: EquationTag<'arena>,
    pub link_target: Option<&'arena str>,
}

/// One cell of a table-like environment: the nodes between two `&` (or `\\`).
pub type TableCell<'arena> = &'arena [&'arena Node<'arena>];

/// One row of a table-like environment: the cells between two `\\`, plus the metadata that is
/// attached to the row as a whole.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TableRow<'arena> {
    /// The equation number and link target of this row, in numbered environments.
    pub label_info: Option<RowLabelInfo<'arena>>,
    /// A line above this row, from a `\hline`/`\hdashline` at the start of the row.
    pub border_top: Option<LineType>,
    /// The alignment forced by a `\shoveleft`/`\shoveright` at the start of the row (`multline`
    /// only).
    pub shove: Option<ColumnAlignment>,
    pub cells: &'arena [TableCell<'arena>],
}

/// The equation number (or custom text from `\tag`) shown next to an equation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EquationTag<'arena> {
    pub text: &'arena str,
    /// `true` if the text should be surrounded by parentheses when rendered.
    /// Tags set with `\tag*` are rendered without parentheses.
    pub parenthesized: bool,
}

#[derive(Debug)]
pub enum AlignmentType<'arena> {
    Predefined(Alignment),
    Custom(&'arena ArraySpec<'arena>),
    /// `multline`, with the total number of rows (which determines the alignment of each row).
    MultLine(usize),
}

impl<'arena> AlignmentType<'arena> {
    pub fn into_generator(self) -> ColumnGenerator<'arena> {
        ColumnGenerator {
            typ: self,
            column_idx: 0,
            row_idx: 0,
            row_border_top: None,
            shove: None,
        }
    }
}

const MTD_OPEN_STYLE: &str = "<mtd style=\"";
const MTD_CLOSE_STYLE: &str = "\">";
const LEFT_ALIGN: &str = "text-align: left;justify-items: start;";
pub const RIGHT_ALIGN: &str = "text-align: right;justify-items: end;";
const PADDING_RIGHT_ZERO: &str = "padding-right: 0;";
const PADDING_LEFT_ZERO: &str = "padding-left: 0;";
const PADDING_TOP_BOTTOM_ZERO: &str = "padding-top: 0;padding-bottom: 0;";
const BORDER_RIGHT_SOLID: &str = "border-right: 0.05em solid currentcolor;";
const BORDER_RIGHT_DASHED: &str = "border-right: 0.05em dashed currentcolor;";
const BORDER_TOP_SOLID: &str = "border-top: 0.05em solid currentcolor;";
const BORDER_TOP_DASHED: &str = "border-top: 0.05em dashed currentcolor;";
const SIMPLE_CENTERED: &str = "<mtd>";

#[derive(Debug)]
pub struct ColumnGenerator<'arena> {
    typ: AlignmentType<'arena>,
    column_idx: usize,
    row_idx: usize,
    /// The top border (from `\hline`/`\hdashline`) applied to every cell of the current row.
    /// MathML `<mtr>` borders aren't rendered by all browsers (notably Firefox), so the line is
    /// drawn per-cell instead.
    row_border_top: Option<LineType>,
    shove: Option<ColumnAlignment>,
}

impl ColumnGenerator<'_> {
    /// Start a row in the table. This must be called before the first cell of every row
    /// (including the first one). It resets the column index to 0, and sets the row index, the top
    /// border and the shove for the row.
    pub fn start_row(
        &mut self,
        row_idx: usize,
        border_top: Option<LineType>,
        shove: Option<ColumnAlignment>,
    ) {
        self.column_idx = 0;
        self.row_idx = row_idx;
        self.row_border_top = border_top;
        self.shove = shove;
    }

    pub fn write_next_mtd(
        &mut self,
        s: &mut String,
        indent_num: usize,
        indentation: Indentation,
    ) -> Result<(), core::fmt::Error> {
        new_line_and_indent(s, indent_num, indentation);
        let column_idx = self.column_idx;
        self.column_idx += 1;
        // Top border (from `\hline`/`\hdashline`) applied to every cell of the current row.
        // When non-empty, the plain `<mtd>` fast paths are replaced by a styled cell, and the
        // border is injected at the start of every other cell's style.
        let border_top = match self.row_border_top {
            None => "",
            Some(LineType::Solid) => BORDER_TOP_SOLID,
            Some(LineType::Dashed) => BORDER_TOP_DASHED,
        };
        match self.typ {
            AlignmentType::Predefined(align) => {
                let is_even = column_idx.is_multiple_of(2);
                match align {
                    Alignment::Cases => {
                        write!(
                            s,
                            "{MTD_OPEN_STYLE}{border_top}{LEFT_ALIGN}{PADDING_RIGHT_ZERO}"
                        )?;
                        if !is_even {
                            write!(s, "padding-left:1em;")?;
                        }
                        write!(s, "{MTD_CLOSE_STYLE}")?;
                    }
                    Alignment::Centered => {
                        write_simple_mtd(s, border_top)?;
                    }
                    Alignment::Alternating => {
                        write!(s, "{MTD_OPEN_STYLE}{border_top}")?;
                        if is_even {
                            write!(s, "{RIGHT_ALIGN}{PADDING_RIGHT_ZERO}")?;
                        } else {
                            write!(s, "{LEFT_ALIGN}{PADDING_LEFT_ZERO}")?;
                        }
                        write!(s, "{MTD_CLOSE_STYLE}")?;
                    }
                }
            }
            AlignmentType::Custom(array_spec) => {
                static DEFAULT_COLUMN_SPEC: ColumnSpecEntry = ColumnSpecEntry::WithContent {
                    alignment: ColumnAlignment::Centered,
                    border_right: None,
                };
                let mut column_spec = array_spec
                    .column_spec
                    .get(column_idx)
                    .unwrap_or(&DEFAULT_COLUMN_SPEC);
                while let ColumnSpecEntry::OnlyLine(line_type) = column_spec {
                    column_spec = array_spec
                        .column_spec
                        .get(self.column_idx)
                        .unwrap_or(&DEFAULT_COLUMN_SPEC);
                    self.column_idx += 1;
                    write!(s, "{MTD_OPEN_STYLE}{border_top}")?;
                    match line_type {
                        LineType::Solid => {
                            write!(s, "{BORDER_RIGHT_SOLID}")?;
                        }
                        LineType::Dashed => {
                            write!(s, "{BORDER_RIGHT_DASHED}")?;
                        }
                    }
                    if array_spec.is_sub {
                        write!(s, "{PADDING_TOP_BOTTOM_ZERO}")?;
                    }
                    write!(s, "padding-left: 0.1em;padding-right: 0.1em;")?;
                    write!(s, "\"></mtd>")?;
                    new_line_and_indent(s, indent_num, indentation);
                }
                match column_spec {
                    ColumnSpecEntry::WithContent {
                        alignment,
                        border_right,
                    } => {
                        if matches!(alignment, ColumnAlignment::Centered)
                            && border_right.is_none()
                            && !array_spec.is_sub
                        {
                            write_simple_mtd(s, border_top)?;
                            return Ok(());
                        }
                        write!(s, "{MTD_OPEN_STYLE}{border_top}")?;
                        match alignment {
                            ColumnAlignment::LeftJustified => {
                                write!(s, "{LEFT_ALIGN}")?;
                            }
                            ColumnAlignment::Centered => {}
                            ColumnAlignment::RightJustified => {
                                write!(s, "{RIGHT_ALIGN}")?;
                            }
                        }
                        match border_right {
                            Some(LineType::Solid) => {
                                write!(s, "{BORDER_RIGHT_SOLID}")?;
                            }
                            Some(LineType::Dashed) => {
                                write!(s, "{BORDER_RIGHT_DASHED}")?;
                            }
                            _ => {}
                        }
                        if array_spec.is_sub {
                            write!(s, "{PADDING_TOP_BOTTOM_ZERO}")?;
                        }
                        write!(s, "{MTD_CLOSE_STYLE}")?;
                    }
                    ColumnSpecEntry::OnlyLine(_) => {}
                }
            }
            AlignmentType::MultLine(num_rows) => {
                let align = if let Some(shove) = self.shove {
                    shove
                } else {
                    // Multline is left-aligned for the first row, right-aligned for the last row,
                    // and centered for all other rows.
                    let row_idx = self.row_idx;
                    if row_idx == 0 {
                        ColumnAlignment::LeftJustified
                    } else if row_idx + 1 == num_rows {
                        ColumnAlignment::RightJustified
                    } else {
                        ColumnAlignment::Centered
                    }
                };
                match align {
                    ColumnAlignment::LeftJustified => {
                        write!(s, "{MTD_OPEN_STYLE}{LEFT_ALIGN}{MTD_CLOSE_STYLE}")?
                    }
                    ColumnAlignment::Centered => write_simple_mtd(s, "")?,
                    ColumnAlignment::RightJustified => {
                        write!(s, "{MTD_OPEN_STYLE}{RIGHT_ALIGN}{MTD_CLOSE_STYLE}")?
                    }
                }
            }
        }
        Ok(())
    }
}

/// Write a centered cell (`<mtd>`) with no other styling, adding a top border if one is set
/// for the current row.
fn write_simple_mtd(s: &mut String, border_top: &str) -> core::fmt::Result {
    if border_top.is_empty() {
        write!(s, "{SIMPLE_CENTERED}")
    } else {
        write!(s, "{MTD_OPEN_STYLE}{border_top}{MTD_CLOSE_STYLE}")
    }
}
