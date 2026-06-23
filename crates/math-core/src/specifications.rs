//! Functions for parsing specifications in LaTeX commands.

use strum_macros::EnumString;

use mathml_renderer::{
    arena::Arena,
    length::{Length, LengthUnit},
    table::{ArraySpec, ColumnAlignment, ColumnSpecEntry, LineType},
};

#[derive(Debug, Clone, Copy, PartialEq, EnumString)]
pub enum LatexUnit {
    /// Point
    #[strum(ascii_case_insensitive)]
    Pt,
    /// Millimeter
    #[strum(ascii_case_insensitive)]
    Mm,
    /// Centimeter
    #[strum(ascii_case_insensitive)]
    Cm,
    /// Inch
    #[strum(ascii_case_insensitive)]
    In,
    /// roughly the height of an 'x' (lowercase) in the current font
    #[strum(ascii_case_insensitive)]
    Ex,
    /// roughly the width of an 'M' (uppercase) in the current font
    #[strum(ascii_case_insensitive)]
    Em,
    /// math unit equal to 1/18 em, where em is taken from the math symbols family
    #[strum(ascii_case_insensitive)]
    Mu,
    /// so-called "special points", a low-level unit of measure where 65536sp=1pt
    #[strum(ascii_case_insensitive)]
    Sp,
}

impl LatexUnit {
    /// Whether this is a math unit (valid after `\mkern`/`\mskip`/`\mspace`)
    /// as opposed to a text unit (valid after `\kern`/`\hskip`/`\hspace`).
    pub const fn is_math_unit(self) -> bool {
        matches!(self, LatexUnit::Mu)
    }

    pub const fn length_with_unit(self, value: f32) -> Length {
        use LengthUnit::*;
        // The conversions are based on the assumption that 1Rem=10pt,
        // which means that we assume the LaTeX document had the font size set to 10pt.
        match self {
            LatexUnit::Pt => Length::new(0.1 * value, Rem),
            LatexUnit::Mm => Length::new(0.28453 * value, Rem),
            LatexUnit::Cm => Length::new(2.8453 * value, Rem),
            LatexUnit::In => Length::new(7.2 * value, Rem),
            LatexUnit::Ex => Length::new(value, Ex),
            LatexUnit::Em => Length::new(value, Em),
            LatexUnit::Mu => Length::new(0.055555556 * value, Em),
            LatexUnit::Sp => Length::new(1.525879e-6 * value, Rem),
        }
    }
}

pub(crate) fn parse_length_specification(s: &str) -> Option<(Length, &str, bool)> {
    let len = s.len();
    // We need at least 2 characters to have a unit.
    let unit_offset = len.checked_sub(2)?;
    // Check whether we can split the string at the unit offset.
    // (This can fail if `unit_offset` is not a valid UTF-8 boundary.)
    let (digits, unit) = s.split_at_checked(unit_offset)?;

    let value = crate::atof::limited_float_parse(digits.trim_ascii_end())?;

    let parsed_unit = LatexUnit::try_from(unit).ok()?;
    Some((
        parsed_unit.length_with_unit(value),
        unit,
        parsed_unit.is_math_unit(),
    ))
}

/// Parses a column specification string in the format "l|c|r" where:
/// - 'l', 'c', 'r' indicate left, center, and right alignment
/// - '|' indicates a vertical line between columns
/// - The specification may begin with a vertical line
pub fn parse_column_specification<'arena>(
    s: &str,
    arena: &'arena Arena,
) -> Option<ArraySpec<'arena>> {
    let mut column_spec = Vec::new();
    let mut border_left: Option<LineType> = None;
    let mut has_content_column = false;

    // We will work with bytes to avoid UTF-8 checks.
    // This is possible because we only match on ASCII characters.
    for ch in s.as_bytes() {
        let ch = *ch;
        match ch {
            b'l' | b'c' | b'r' => {
                let alignment = match ch {
                    b'l' => ColumnAlignment::LeftJustified,
                    b'c' => ColumnAlignment::Centered,
                    b'r' => ColumnAlignment::RightJustified,
                    _ => unreachable!(),
                };

                column_spec.push(ColumnSpecEntry::WithContent {
                    alignment,
                    border_right: None,
                });
                has_content_column = true;
            }
            b'|' | b':' => {
                let line_type = match ch {
                    b'|' => LineType::Solid,
                    b':' => LineType::Dashed,
                    _ => unreachable!(),
                };
                if let Some(last) = column_spec.last_mut() {
                    // If the last column was a content column, we need to add a line type.
                    // If it is already set, we add a new element to the column spec vector.
                    if let ColumnSpecEntry::WithContent {
                        border_right: last_line_type @ None,
                        ..
                    } = last
                    {
                        *last_line_type = Some(line_type);
                    } else {
                        column_spec.push(ColumnSpecEntry::OnlyLine(line_type))
                    }
                } else {
                    // Nothing has been added yet, so this is the first column.
                    if border_left.is_none() {
                        border_left = Some(line_type);
                    } else {
                        // If there already is a `border_left`, we have a double line.
                        column_spec.push(ColumnSpecEntry::OnlyLine(line_type))
                    }
                }
            }
            _ if ch.is_ascii_whitespace() => {
                // Skip whitespace, already handled by next()
            }
            _ => {
                // Invalid character, return None
                return None;
            }
        }
    }

    if column_spec.is_empty() || !has_content_column {
        return None;
    }

    Some(ArraySpec {
        border_left,
        border_top: None,
        is_sub: false,
        column_spec: arena.alloc_column_spec(column_spec.as_slice()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_column_specs() {
        let arena = Arena::new();
        assert_eq!(parse_column_specification("", &arena), None);
        assert_eq!(parse_column_specification("|", &arena), None);
        assert_eq!(parse_column_specification("||", &arena), None);
        assert_eq!(parse_column_specification("x", &arena), None);
        assert_eq!(parse_column_specification("x|c", &arena), None);
        assert_eq!(parse_column_specification("c|x", &arena), None);
        assert_eq!(parse_column_specification("cx", &arena), None);
        assert_eq!(parse_column_specification("👍🏽c", &arena), None);
        assert_eq!(parse_column_specification("|c|👍🏽", &arena), None);
    }

    #[test]
    fn column_parse_simple() {
        let arena = Arena::new();
        let spec = parse_column_specification("l|c|r", &arena).unwrap();
        assert!(spec.border_left.is_none());
        assert_eq!(spec.column_spec.len(), 3);
        assert!(matches!(
            spec.column_spec[0],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::LeftJustified,
                border_right: Some(LineType::Solid)
            }
        ));
        assert!(matches!(
            spec.column_spec[1],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: Some(LineType::Solid)
            }
        ));
        assert!(matches!(
            spec.column_spec[2],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::RightJustified,
                border_right: None
            }
        ));
    }

    #[test]
    fn column_parse_line_at_beginning() {
        let arena = Arena::new();
        let spec = parse_column_specification("|ccc", &arena).unwrap();
        assert!(matches!(spec.border_left, Some(LineType::Solid)));
        assert_eq!(spec.column_spec.len(), 3);
        assert!(matches!(
            spec.column_spec[0],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: None
            }
        ));
        assert!(matches!(
            spec.column_spec[1],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: None
            }
        ));
        assert!(matches!(
            spec.column_spec[2],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: None
            }
        ));
    }

    #[test]
    fn column_parse_multiple_line_at_beginning() {
        let arena = Arena::new();
        let spec = parse_column_specification("   | ||c", &arena).unwrap();
        assert!(matches!(spec.border_left, Some(LineType::Solid)));
        assert_eq!(spec.column_spec.len(), 3);
        assert!(matches!(
            spec.column_spec[0],
            ColumnSpecEntry::OnlyLine(LineType::Solid)
        ));
        assert!(matches!(
            spec.column_spec[1],
            ColumnSpecEntry::OnlyLine(LineType::Solid)
        ));
        assert!(matches!(
            spec.column_spec[2],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: None
            }
        ));
    }

    #[test]
    fn column_parse_with_spaces() {
        let arena = Arena::new();
        let spec = parse_column_specification(" c   : |   c|    : |      c ", &arena).unwrap();
        assert!(spec.border_left.is_none());
        assert_eq!(spec.column_spec.len(), 6);
        assert!(matches!(
            spec.column_spec[0],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: Some(LineType::Dashed)
            }
        ));
        assert!(matches!(
            spec.column_spec[1],
            ColumnSpecEntry::OnlyLine(LineType::Solid)
        ));
        assert!(matches!(
            spec.column_spec[2],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: Some(LineType::Solid)
            }
        ));
        assert!(matches!(
            spec.column_spec[3],
            ColumnSpecEntry::OnlyLine(LineType::Dashed)
        ));
        assert!(matches!(
            spec.column_spec[4],
            ColumnSpecEntry::OnlyLine(LineType::Solid)
        ));
        assert!(matches!(
            spec.column_spec[5],
            ColumnSpecEntry::WithContent {
                alignment: ColumnAlignment::Centered,
                border_right: None
            }
        ));
    }

    #[test]
    fn latex_unit() {
        assert_eq!(LatexUnit::try_from("CM").unwrap(), LatexUnit::Cm);
        assert_eq!(LatexUnit::try_from("mM").unwrap(), LatexUnit::Mm);
    }

    #[test]
    fn round_trip_em() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
                .0
                .push_to_string(&mut output);
            assert_eq!(s, &output);
        }
        for i in 1..10 {
            rt(&format!("{i}em"));
            rt(&format!("0.{i}em"));
            rt(&format!("{i}.25em"));
            rt(&format!("{i}.75em"));
            for j in 1..10 {
                rt(&format!("{i}{j}em"));
                rt(&format!("{i}.{j}em"));
            }
        }
    }

    #[test]
    fn round_trip_negative_em() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
                .0
                .push_to_string(&mut output);
            assert_eq!(s, &output);
        }
        for i in 1..10 {
            rt(&format!("-{i}em"));
            rt(&format!("-0.{i}em"));
            rt(&format!("-{i}.25em"));
            rt(&format!("-{i}.75em"));
            for j in 1..10 {
                rt(&format!("-{i}{j}em"));
                rt(&format!("{i}.{j}em"));
            }
        }
    }
}
