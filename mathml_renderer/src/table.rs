#[cfg(feature = "serde")]
use serde::Serialize;

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

impl<'arena> Iterator for ColumnGenerator<'arena> {
    type Item = &'static str;

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
                                r#"<mtd style="text-align: -webkit-left; text-align: -moz-left; padding-right: 0">"#,
                            )
                        } else {
                            Some(
                                r#"<mtd style="text-align: -webkit-left; text-align: -moz-left; padding-right: 0; padding-left: 1em">"#,
                            )
                        }
                    }
                    Alignment::Centered => Some("<mtd>"),
                    Alignment::Alternating => {
                        if is_even {
                            Some(
                                r#"<mtd style="text-align: -webkit-right; text-align: -moz-right; padding-right: 0">"#,
                            )
                        } else {
                            Some(
                                r#"<mtd style="text-align: -webkit-left; text-align: -moz-left; padding-left: 0">"#,
                            )
                        }
                    }
                }
            }
            AlignmentType::Custom(array_spec) => {
                let column_spec = array_spec.column_spec.get(column_idx)?;
                match column_spec.alignment {
                    Some(ColumnAlignment::LeftJustified) => {
                        Some(r#"<mtd style="text-align: -webkit-left; text-align: -moz-left">"#)
                    }
                    Some(ColumnAlignment::Centered) => {
                        if column_spec.with_line {
                            Some(r#"<mtd style="border-right: 1px solid black">"#)
                        } else {
                            Some("<mtd>")
                        }
                    }
                    Some(ColumnAlignment::RightJustified) => {
                        Some(r#"<mtd style="text-align: -webkit-right; text-align: -moz-right">"#)
                    }
                    None => {
                        self.column_idx += 1;
                        if column_spec.with_line {
                            Some(
                                r#"<mtd style="border-right: 1px solid black; padding-left: 0.2em; padding-right: 0.2em"></mtd><mtd>"#,
                            )
                        } else {
                            Some("<mtd></mtd><mtd>")
                        }
                    }
                }
            }
        }
    }
}
