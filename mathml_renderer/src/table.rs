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

pub fn center_alignment(_counter: usize) -> Option<&'static str> {
    Some("<mtd>")
}

pub fn cases_alignment(counter: usize) -> Option<&'static str> {
    let result = if counter % 2 == 0 {
        r#"<mtd style="text-align: -webkit-left; text-align: -moz-left; padding-right: 0">"#
    } else {
        "<mtd style=\"text-align: -webkit-left; text-align: -moz-left; padding-right: 0; padding-left: 1em\">"
    };
    Some(result)
}

pub fn alternating_alignment(counter: usize) -> Option<&'static str> {
    let result = if counter % 2 == 0 {
        r#"<mtd style="text-align: -webkit-right; text-align: -moz-right; padding-right: 0">"#
    } else {
        "<mtd style=\"text-align: -webkit-left; text-align: -moz-left; padding-left: 0\">"
    };
    Some(result)
}
