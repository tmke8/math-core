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

impl<'arena> ArraySpec<'arena> {
    #[inline]
    fn get_opening(&self, counter: usize) -> Option<&'static str> {
        self.column_spec
            .get(counter)
            .and_then(|column_spec| match column_spec.alignment {
                Some(ColumnAlignment::LeftJustified) => {
                    Some(r#"<mtd style="text-align: -webkit-left; text-align: -moz-left">"#)
                }
                Some(ColumnAlignment::Centered) => Some("<mtd>"),
                Some(ColumnAlignment::RightJustified) => {
                    Some(r#"<mtd style="text-align: -webkit-right; text-align: -moz-right">"#)
                }
                None => None,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Align {
    Center,
    Left,
    Alternating,
}

pub enum MtdOpening<'arena> {
    Predefined(Align),
    Custom(&'arena ArraySpec<'arena>),
}

impl<'arena> MtdOpening<'arena> {
    pub fn get_opening(&self, counter: usize) -> Option<&'static str> {
        match self {
            MtdOpening::Predefined(align) => {
                let is_even = counter % 2 == 0;
                match align {
                    Align::Left => {
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
                    Align::Center => Some("<mtd>"),
                    Align::Alternating => {
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
            MtdOpening::Custom(array_spec) => array_spec.get_opening(counter),
        }
    }
}
