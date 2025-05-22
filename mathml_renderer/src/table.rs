#[cfg(feature = "serde")]
use serde::Serialize;

use crate::fmt::StrJoiner;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ColumnAlignment {
    LeftJustified,
    Centered,
    RightJustified,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ColumnSpec {
    pub alignment: Option<ColumnAlignment>,
    pub with_line: bool,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ArraySpec<'arena> {
    pub begins_with_line: bool,
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
}

const MTD_OPEN_STYLE: &'static str = "<mtd style=\"";
const MTD_CLOSE_STYLE: &'static str = "\">";
const LEFT_ALIGN: &'static str = "text-align: -webkit-left;text-align: -moz-left;";
const RIGHT_ALIGN: &'static str = "text-align: -webkit-right;text-align: -moz-right;";
const PADDING_RIGHT_ZERO: &'static str = "padding-right: 0;";
const PADDING_LEFT_ZERO: &'static str = "padding-left: 0;";
const BORDER_RIGHT: &'static str = "border-right: 1px solid black;";
pub const SIMPLE_CENTERED: &'static StrJoiner = StrJoiner::from_slice(&["<mtd>"]);

impl<'arena> Iterator for ColumnGenerator<'arena> {
    type Item = &'static StrJoiner;

    fn next(&mut self) -> Option<Self::Item> {
        let column_idx = self.column_idx;
        self.column_idx += 1;
        match self.typ {
            AlignmentType::Predefined(align) => {
                let is_even = column_idx % 2 == 0;
                match align {
                    Alignment::Cases => {
                        if is_even {
                            Some(
                                StrJoiner::from_slice(&[MTD_OPEN_STYLE, LEFT_ALIGN, PADDING_RIGHT_ZERO, MTD_CLOSE_STYLE])
                            )
                        } else {
                            Some(
                                StrJoiner::from_slice(&[MTD_OPEN_STYLE, LEFT_ALIGN, PADDING_RIGHT_ZERO, "padding-left:1em;", MTD_CLOSE_STYLE])
                            )
                        }
                    }
                    Alignment::Centered => Some(SIMPLE_CENTERED),
                    Alignment::Alternating => {
                        if is_even {
                            Some(
                                StrJoiner::from_slice(&[MTD_OPEN_STYLE, RIGHT_ALIGN, PADDING_RIGHT_ZERO, MTD_CLOSE_STYLE])
                            )
                        } else {
                            Some(
                                StrJoiner::from_slice(&[MTD_OPEN_STYLE, LEFT_ALIGN, PADDING_LEFT_ZERO, MTD_CLOSE_STYLE])
                            )
                        }
                    }
                }
            }
            AlignmentType::Custom(array_spec) => {
                let column_spec = array_spec.column_spec.get(column_idx)?;
                match column_spec.alignment {
                    Some(ColumnAlignment::LeftJustified) => {
                        Some(StrJoiner::from_slice(&[MTD_OPEN_STYLE, LEFT_ALIGN, MTD_CLOSE_STYLE]))
                    }
                    Some(ColumnAlignment::Centered) => {
                        if column_spec.with_line {
                            Some(StrJoiner::from_slice(&[MTD_OPEN_STYLE, BORDER_RIGHT, MTD_CLOSE_STYLE]))
                        } else {
                            Some(SIMPLE_CENTERED)
                        }
                    }
                    Some(ColumnAlignment::RightJustified) => {
                        Some(StrJoiner::from_slice(&[MTD_OPEN_STYLE, RIGHT_ALIGN, MTD_CLOSE_STYLE]))
                    }
                    None => {
                        self.column_idx += 1;
                        if column_spec.with_line {
                            Some(StrJoiner::from_slice(&[MTD_OPEN_STYLE, BORDER_RIGHT, "padding-left: 0.2em;padding-right: 0.2em;", "\"></mtd><mtd>"]))
                        } else {
                            Some(StrJoiner::from_slice(&["<mtd></mtd><mtd>"]))
                        }
                    }
                }
            }
        }
    }
}
