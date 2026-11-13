use alloc::string::String;
use core::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::ast::{Indentation, Node};
use crate::attribute::Style;
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
    /// Centered columns which are set closer together than usual, both horizontally and
    /// vertically, as in the `smallmatrix` environment.
    Small,
}

/// The kind of a [`Node::Table`], which determines how the table is rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum TableKind {
    /// A table inside a formula, like a matrix or the `aligned` and `cases` environments.
    Inline {
        align: Alignment,
        style: Option<Style>,
    },
    /// An equation array like the `align` environment, which spans the full width and has
    /// room for equation numbers.
    EquationArray(Alignment),
    /// The `multline` environment.
    MultLine,
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
    /// A line below this row. Only set on the last row, for an `\hline`/`\hdashline` right
    /// before the end of the environment; all other lines are the top border of the row below.
    pub border_bottom: Option<LineType>,
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

/// How the cells of a table are aligned, which determines the opening `<mtd>` tag of each cell.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AlignmentType<'arena> {
    Predefined(Alignment),
    Custom(&'arena ArraySpec<'arena>),
    /// `multline`, with the total number of rows (which determines the alignment of each row).
    MultLine(usize),
}

const MTD_OPEN_STYLE: &str = "<mtd style=\"";
const MTD_CLOSE_STYLE: &str = "\">";
const LEFT_ALIGN: &str = "text-align: left;justify-items: start;";
pub const RIGHT_ALIGN: &str = "text-align: right;justify-items: end;";
const PADDING_RIGHT_ZERO: &str = "padding-right: 0;";
const PADDING_LEFT_ZERO: &str = "padding-left: 0;";
const PADDING_TOP_BOTTOM_ZERO: &str = "padding-top: 0;padding-bottom: 0;";
/// The padding of the first column of an [`Alignment::Small`] table.
///
/// The baselines of the rows of amsmath's `smallmatrix` are 6pt apart (at a font size of 10pt),
/// unless the content is too tall for that, in which case the rows are `\lineskip` (1.5pt) apart.
/// A row in MathML is as tall as its content, so only the second case can be reproduced; it is
/// the common one, since digits and letters with ascenders are already too tall. Rows with only
/// short content (like `a`, `c` or `x`) end up closer together than in LaTeX. See
/// <https://github.com/tmke8/math-core/issues/1282>.
///
/// The cells are in script size, which is 0.7 times the text size, so the 1.5pt are 0.2em there,
/// split between the top and the bottom.
const SMALL_PADDING_FIRST: &str = "padding: 0.1em 0;";
/// The padding of all other columns of an [`Alignment::Small`] table.
///
/// amsmath separates the columns of `smallmatrix` by a `\thickspace` (0.2777em) of the text font,
/// which is 0.4em in script size. The space is put to the left of each column, as in amsmath.
const SMALL_PADDING_OTHER: &str = "padding: 0.1em 0 0 0.4em;";
/// The padding at the left and right edges of an `array`.
///
/// LaTeX puts `\arraycolsep` (5pt, which is 0.5em at a font size of 10pt) on both sides of every
/// column of an `array`, including the outermost ones. The style sheet removes the padding at
/// the edges of all tables, which is right for `matrix`, `aligned` etc., so arrays have to put
/// it back.
const ARRAY_PADDING_LEFT: &str = "padding-left: 0.5em;";
const ARRAY_PADDING_RIGHT: &str = "padding-right: 0.5em;";
const BORDER_RIGHT_SOLID: &str = "border-right: 0.05em solid currentcolor;";
const BORDER_RIGHT_DASHED: &str = "border-right: 0.05em dashed currentcolor;";
const BORDER_TOP_SOLID: &str = "border-top: 0.05em solid currentcolor;";
const BORDER_TOP_DASHED: &str = "border-top: 0.05em dashed currentcolor;";
const BORDER_BOTTOM_SOLID: &str = "border-bottom: 0.05em solid currentcolor;";
const BORDER_BOTTOM_DASHED: &str = "border-bottom: 0.05em dashed currentcolor;";
const SIMPLE_CENTERED: &str = "<mtd>";

/// The style for the top and bottom borders (from `\hline`/`\hdashline`) of a row.
///
/// MathML `<mtr>` borders aren't rendered by all browsers (notably Firefox), so the lines are
/// drawn on every cell of the row instead. When non-empty, the plain `<mtd>` fast paths are
/// replaced by a styled cell, and the borders are injected at the start of every other cell's
/// style.
#[derive(Clone, Copy)]
struct RowBorders {
    top: &'static str,
    bottom: &'static str,
}

impl RowBorders {
    fn new(row: &TableRow<'_>) -> Self {
        Self {
            top: match row.border_top {
                None => "",
                Some(LineType::Solid) => BORDER_TOP_SOLID,
                Some(LineType::Dashed) => BORDER_TOP_DASHED,
            },
            bottom: match row.border_bottom {
                None => "",
                Some(LineType::Solid) => BORDER_BOTTOM_SOLID,
                Some(LineType::Dashed) => BORDER_BOTTOM_DASHED,
            },
        }
    }

    fn is_empty(self) -> bool {
        self.top.is_empty() && self.bottom.is_empty()
    }
}

impl core::fmt::Display for RowBorders {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.top)?;
        f.write_str(self.bottom)
    }
}

/// What [`AlignmentType::write_mtd`] needs to know about the row that a cell is in.
///
/// This is the same for all cells of a row, so it is built once per row.
pub(crate) struct RowContext {
    /// The row index.
    idx: usize,
    borders: RowBorders,
    /// See [`TableRow::shove`].
    shove: Option<ColumnAlignment>,
    /// The indentation of the `<mtd>` elements of the row.
    indent_num: usize,
    indentation: Indentation,
}

impl RowContext {
    pub(crate) fn new(
        idx: usize,
        row: &TableRow<'_>,
        indent_num: usize,
        indentation: Indentation,
    ) -> Self {
        Self {
            idx,
            borders: RowBorders::new(row),
            shove: row.shove,
            indent_num,
            indentation,
        }
    }
}

impl AlignmentType<'_> {
    /// Write the opening `<mtd>` tag of the cell at `column_idx` in the row described by `row`,
    /// and return the column index of the next cell in the row.
    ///
    /// The returned index is usually `column_idx + 1`, but for arrays it is an index into the
    /// column spec, which can be different from the cell index: entries that are only a line (like
    /// the second `|` in `{c||c}`) don't correspond to a cell of the input, so for each of those,
    /// an empty `<mtd>` that draws the line is written first.
    ///
    /// `is_last_cell` says whether this is the last cell of the row.
    pub(crate) fn write_mtd(
        self,
        s: &mut String,
        row: &RowContext,
        mut column_idx: usize,
        is_last_cell: bool,
    ) -> Result<usize, core::fmt::Error> {
        let borders = row.borders;
        match self {
            AlignmentType::Predefined(align) => {
                let is_even = column_idx.is_multiple_of(2);
                match align {
                    Alignment::Cases => {
                        write!(
                            s,
                            "{MTD_OPEN_STYLE}{borders}{LEFT_ALIGN}{PADDING_RIGHT_ZERO}"
                        )?;
                        if !is_even {
                            write!(s, "padding-left:1em;")?;
                        }
                        write!(s, "{MTD_CLOSE_STYLE}")?;
                    }
                    Alignment::Centered => write_simple_mtd(s, borders)?,
                    Alignment::Small => {
                        let padding = if column_idx == 0 {
                            SMALL_PADDING_FIRST
                        } else {
                            SMALL_PADDING_OTHER
                        };
                        write!(s, "{MTD_OPEN_STYLE}{borders}{padding}{MTD_CLOSE_STYLE}")?;
                    }
                    Alignment::Alternating => {
                        write!(s, "{MTD_OPEN_STYLE}{borders}")?;
                        if is_even {
                            write!(s, "{RIGHT_ALIGN}{PADDING_RIGHT_ZERO}")?;
                        } else {
                            write!(s, "{LEFT_ALIGN}{PADDING_LEFT_ZERO}")?;
                        }
                        write!(s, "{MTD_CLOSE_STYLE}")?;
                    }
                }
                column_idx += 1;
            }
            AlignmentType::Custom(array_spec) => {
                static DEFAULT_COLUMN_SPEC: ColumnSpecEntry = ColumnSpecEntry::WithContent {
                    alignment: ColumnAlignment::Centered,
                    border_right: None,
                };
                // Subarrays have no padding at the edges. In other arrays, the cell needs it if
                // it is the first or the last `<mtd>` of the row; a line-only `<mtd>` next to it
                // brings its own padding and leaves this cell with the default one.
                let mut pad_left = column_idx == 0 && !array_spec.is_sub;
                let (alignment, border_right) = loop {
                    let entry = array_spec
                        .column_spec
                        .get(column_idx)
                        .copied()
                        .unwrap_or(DEFAULT_COLUMN_SPEC);
                    column_idx += 1;
                    match entry {
                        ColumnSpecEntry::WithContent {
                            alignment,
                            border_right,
                        } => break (alignment, border_right),
                        ColumnSpecEntry::OnlyLine(line_type) => {
                            write_line_mtd(s, borders, line_type, array_spec.is_sub)?;
                            new_line_and_indent(s, row.indent_num, row.indentation);
                            pad_left = false;
                        }
                    }
                };
                // If this is the last cell, the line-only entries that follow it are written by
                // `write_trailing_lines`.
                let pad_right = is_last_cell
                    && !array_spec.is_sub
                    && !matches!(
                        array_spec.column_spec.get(column_idx),
                        Some(ColumnSpecEntry::OnlyLine(_))
                    );
                if matches!(alignment, ColumnAlignment::Centered)
                    && border_right.is_none()
                    && !array_spec.is_sub
                    && !pad_left
                    && !pad_right
                {
                    write_simple_mtd(s, borders)?;
                    return Ok(column_idx);
                }
                write!(s, "{MTD_OPEN_STYLE}{borders}")?;
                match alignment {
                    ColumnAlignment::LeftJustified => write!(s, "{LEFT_ALIGN}")?,
                    ColumnAlignment::Centered => {}
                    ColumnAlignment::RightJustified => write!(s, "{RIGHT_ALIGN}")?,
                }
                match border_right {
                    Some(LineType::Solid) => write!(s, "{BORDER_RIGHT_SOLID}")?,
                    Some(LineType::Dashed) => write!(s, "{BORDER_RIGHT_DASHED}")?,
                    None => {}
                }
                if array_spec.is_sub {
                    write!(s, "{PADDING_TOP_BOTTOM_ZERO}")?;
                }
                if pad_left {
                    write!(s, "{ARRAY_PADDING_LEFT}")?;
                }
                if pad_right {
                    write!(s, "{ARRAY_PADDING_RIGHT}")?;
                }
                write!(s, "{MTD_CLOSE_STYLE}")?;
            }
            AlignmentType::MultLine(num_rows) => {
                // Multline is left-aligned for the first row, right-aligned for the last row,
                // and centered for all other rows, unless overridden by
                // `\shoveleft`/`\shoveright`.
                let align = row.shove.unwrap_or(if row.idx == 0 {
                    ColumnAlignment::LeftJustified
                } else if row.idx + 1 == num_rows {
                    ColumnAlignment::RightJustified
                } else {
                    ColumnAlignment::Centered
                });
                match align {
                    ColumnAlignment::LeftJustified => {
                        write!(s, "{MTD_OPEN_STYLE}{LEFT_ALIGN}{MTD_CLOSE_STYLE}")?;
                    }
                    ColumnAlignment::Centered => write!(s, "{SIMPLE_CENTERED}")?,
                    ColumnAlignment::RightJustified => {
                        write!(s, "{MTD_OPEN_STYLE}{RIGHT_ALIGN}{MTD_CLOSE_STYLE}")?;
                    }
                }
                column_idx += 1;
            }
        }
        Ok(column_idx)
    }

    /// Write the line-only columns that directly follow the last cell of a row, like the second
    /// `|` in `{c||}`. There is no cell after them whose `<mtd>` could bring them along, so they
    /// have to be written after the last cell has been closed.
    ///
    /// `column_idx` is the index that [`AlignmentType::write_mtd`] returned for the last cell.
    pub(crate) fn write_trailing_lines(
        self,
        s: &mut String,
        row: &RowContext,
        column_idx: usize,
    ) -> core::fmt::Result {
        let AlignmentType::Custom(array_spec) = self else {
            return Ok(());
        };
        // `column_idx` is past the end if the row has more cells than the column spec.
        let remaining = array_spec.column_spec.get(column_idx..).unwrap_or_default();
        for entry in remaining {
            let ColumnSpecEntry::OnlyLine(line_type) = *entry else {
                break;
            };
            new_line_and_indent(s, row.indent_num, row.indentation);
            write_line_mtd(s, row.borders, line_type, array_spec.is_sub)?;
        }
        Ok(())
    }
}

/// Write an empty cell (`<mtd>`) that only draws a vertical line, for a
/// [`ColumnSpecEntry::OnlyLine`].
fn write_line_mtd(
    s: &mut String,
    borders: RowBorders,
    line_type: LineType,
    is_sub: bool,
) -> core::fmt::Result {
    write!(s, "{MTD_OPEN_STYLE}{borders}")?;
    match line_type {
        LineType::Solid => write!(s, "{BORDER_RIGHT_SOLID}")?,
        LineType::Dashed => write!(s, "{BORDER_RIGHT_DASHED}")?,
    }
    if is_sub {
        write!(s, "{PADDING_TOP_BOTTOM_ZERO}")?;
    }
    write!(s, "padding-left: 0.1em;padding-right: 0.1em;\"></mtd>")
}

/// Write a centered cell (`<mtd>`) with no other styling, adding the borders of the current row
/// if it has any.
fn write_simple_mtd(s: &mut String, borders: RowBorders) -> core::fmt::Result {
    if borders.is_empty() {
        write!(s, "{SIMPLE_CENTERED}")
    } else {
        write!(s, "{MTD_OPEN_STYLE}{borders}{MTD_CLOSE_STYLE}")
    }
}
