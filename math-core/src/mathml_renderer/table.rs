use std::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

use super::fmt::new_line_and_indent;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ColumnAlignment {
    LeftJustified = 0,
    Centered = 1,
    RightJustified = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LineType {
    Solid = 3,
    Dashed = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ColumnSpec {
    WithContent(ColumnAlignment, Option<LineType>),
    OnlyLine(LineType),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ArraySpec<'arena> {
    pub beginning_line: Option<LineType>,
    pub is_sub: bool,
    pub column_spec: &'arena [ColumnSpec],
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Alignment {
    Centered,
    Cases,
    Alternating,
}

enum AlignmentType<'arena> {
    Predefined(Alignment),
    Custom(&'arena ArraySpec<'arena>),
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
const SIMPLE_CENTERED: &str = "<mtd>";

pub struct ColumnGenerator<'arena> {
    typ: AlignmentType<'arena>,
    column_idx: usize,
}

impl<'arena> ColumnGenerator<'arena> {
    pub fn new_predefined(align: Alignment) -> Self {
        ColumnGenerator {
            typ: AlignmentType::Predefined(align),
            column_idx: 0,
        }
    }

    pub fn new_custom(array_spec: &'arena ArraySpec<'arena>) -> Self {
        ColumnGenerator {
            typ: AlignmentType::Custom(array_spec),
            column_idx: 0,
        }
    }

    pub fn reset_columns(&mut self) {
        self.column_idx = 0;
    }

    pub fn write_next_mtd(&mut self, s: &mut String, indent_num: usize) {
        new_line_and_indent(s, indent_num);
        let column_idx = self.column_idx;
        self.column_idx += 1;
        match self.typ {
            AlignmentType::Predefined(align) => {
                let is_even = column_idx.is_multiple_of(2);
                match align {
                    Alignment::Cases => {
                        let _ = write!(s, "{MTD_OPEN_STYLE}{LEFT_ALIGN}{PADDING_RIGHT_ZERO}");
                        if !is_even {
                            let _ = write!(s, "padding-left:1em;");
                        }
                        let _ = write!(s, "{MTD_CLOSE_STYLE}");
                    }
                    Alignment::Centered => {
                        let _ = write!(s, "{SIMPLE_CENTERED}");
                    }
                    Alignment::Alternating => {
                        let _ = write!(s, "{MTD_OPEN_STYLE}");
                        if is_even {
                            let _ = write!(s, "{RIGHT_ALIGN}{PADDING_RIGHT_ZERO}");
                        } else {
                            let _ = write!(s, "{LEFT_ALIGN}{PADDING_LEFT_ZERO}");
                        }
                        let _ = write!(s, "{MTD_CLOSE_STYLE}");
                    }
                }
            }
            AlignmentType::Custom(array_spec) => {
                let mut column_spec = array_spec
                    .column_spec
                    .get(column_idx)
                    .unwrap_or(&ColumnSpec::WithContent(ColumnAlignment::Centered, None));
                while let ColumnSpec::OnlyLine(line_type) = column_spec {
                    column_spec = array_spec
                        .column_spec
                        .get(self.column_idx)
                        .unwrap_or(&ColumnSpec::WithContent(ColumnAlignment::Centered, None));
                    self.column_idx += 1;
                    let _ = write!(s, "{MTD_OPEN_STYLE}");
                    match line_type {
                        LineType::Solid => {
                            let _ = write!(s, "{BORDER_RIGHT_SOLID}");
                        }
                        LineType::Dashed => {
                            let _ = write!(s, "{BORDER_RIGHT_DASHED}");
                        }
                    }
                    if array_spec.is_sub {
                        let _ = write!(s, "{PADDING_TOP_BOTTOM_ZERO}");
                    }
                    let _ = write!(s, "padding-left: 0.1em;padding-right: 0.1em;");
                    let _ = write!(s, "\"></mtd>");
                    new_line_and_indent(s, indent_num);
                }
                match column_spec {
                    ColumnSpec::WithContent(alignment, line_type) => {
                        if matches!(alignment, ColumnAlignment::Centered)
                            && line_type.is_none()
                            && !array_spec.is_sub
                        {
                            let _ = write!(s, "{SIMPLE_CENTERED}");
                            return;
                        }
                        let _ = write!(s, "{MTD_OPEN_STYLE}");
                        match alignment {
                            ColumnAlignment::LeftJustified => {
                                let _ = write!(s, "{LEFT_ALIGN}");
                            }
                            ColumnAlignment::Centered => {}
                            ColumnAlignment::RightJustified => {
                                let _ = write!(s, "{RIGHT_ALIGN}");
                            }
                        }
                        match line_type {
                            Some(LineType::Solid) => {
                                let _ = write!(s, "{BORDER_RIGHT_SOLID}");
                            }
                            Some(LineType::Dashed) => {
                                let _ = write!(s, "{BORDER_RIGHT_DASHED}");
                            }
                            _ => {}
                        }
                        if array_spec.is_sub {
                            let _ = write!(s, "{PADDING_TOP_BOTTOM_ZERO}");
                        }
                        let _ = write!(s, "{MTD_CLOSE_STYLE}");
                    }
                    ColumnSpec::OnlyLine(_) => {}
                }
            }
        };
    }
}
