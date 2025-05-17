//! Functions for parsing specifications in LaTeX commands.

use strum_macros::EnumString;

use mathml_renderer::{
    arena::Arena,
    length::{Length, LengthUnit},
    table::{ArraySpec, ColumnAlignment, ColumnSpec},
};

#[derive(Debug, Clone, Copy, PartialEq, EnumString)]
pub enum LaTeXUnit {
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

impl LaTeXUnit {
    pub const fn length_with_unit(self, value: f32) -> Length {
        use LengthUnit::*;
        // The conversions are based on the assumption that 1Rem=10pt,
        // which means that we assume the LaTeX document had the font size set to 10pt.
        match self {
            LaTeXUnit::Pt => Length::new(0.1 * value, Rem),
            LaTeXUnit::Mm => Length::new(0.28453 * value, Rem),
            LaTeXUnit::Cm => Length::new(2.8453 * value, Rem),
            LaTeXUnit::In => Length::new(7.2 * value, Rem),
            LaTeXUnit::Ex => Length::new(value, Ex),
            LaTeXUnit::Em => Length::new(value, Em),
            LaTeXUnit::Mu => Length::new(0.055555556 * value, Em),
            LaTeXUnit::Sp => Length::new(1.525879e-6 * value, Rem),
        }
    }
}

pub(crate) fn parse_length_specification(s: &str) -> Option<Length> {
    let len = s.len();
    // We need at least 2 characters to have a unit.
    let unit_offset = len.checked_sub(2)?;
    // Check whether we can split the string at the unit offset.
    // (This can fail if `unit_offset` is not a valid UTF-8 boundary.)
    let (digits, unit) = s.split_at_checked(unit_offset)?;

    let value = crate::atof::limited_float_parse(digits.trim_end())?;

    let parsed_unit = LaTeXUnit::try_from(unit).ok()?;
    Some(parsed_unit.length_with_unit(value))
}

/// Parses a column specification string in the format "l|c|r" where:
/// - 'l', 'c', 'r' indicate left, center, and right alignment
/// - '|' indicates a vertical line between columns
/// - The specification may begin with a vertical line
pub fn parse_column_specification<'arena>(
    s: &str,
    arena: &'arena Arena,
) -> Option<ArraySpec<'arena>> {
    if !s.is_ascii() {
        return None;
    }

    // Now that we have established that the string is ASCII, we can safely use `trim_start_ascii`.
    let s = s.trim_ascii_start();

    if s.is_empty() {
        return None;
    }

    // Check if the string begins with a line
    let begins_with_line = s.starts_with('|');

    // We will work with bytes to avoid UTF-8 checks. This is fine because everything is ASCII.
    let mut chars = s.as_bytes().iter().peekable();

    // Skip the first '|' if it exists
    if begins_with_line {
        chars.next();
    }

    let mut column_spec = Vec::new();
    let mut has_content_column = false;

    while let Some(&ch) = chars.next() {
        match ch {
            b'l' | b'c' | b'r' => {
                let alignment = match ch {
                    b'l' => Some(ColumnAlignment::LeftJustified),
                    b'c' => Some(ColumnAlignment::Centered),
                    b'r' => Some(ColumnAlignment::RightJustified),
                    _ => unreachable!(),
                };

                // Skip all whitespace
                while chars.peek().map_or(false, |&&c| c.is_ascii_whitespace()) {
                    chars.next();
                }

                // Check if the next character is a vertical line
                let with_line = chars.peek().map_or(false, |&&c| c == b'|');
                if with_line {
                    chars.next(); // Skip over the vertical line
                }

                column_spec.push(ColumnSpec {
                    alignment,
                    with_line,
                });
                has_content_column = true;
            }
            b'|' => {
                // This is a vertical line without an alignment character before it
                // Add an empty column with a vertical line
                column_spec.push(ColumnSpec {
                    alignment: None,
                    with_line: true,
                });
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
        begins_with_line,
        column_spec: arena.inner.alloc_slice(column_spec.as_slice()),
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
    }

    #[test]
    fn column_parse_simple() {
        let arena = Arena::new();
        let spec = parse_column_specification("l|c|r", &arena).unwrap();
        assert_eq!(spec.begins_with_line, false);
        assert_eq!(spec.column_spec.len(), 3);
        assert_eq!(
            spec.column_spec[0].alignment,
            Some(ColumnAlignment::LeftJustified)
        );
        assert_eq!(spec.column_spec[0].with_line, true);
        assert_eq!(
            spec.column_spec[1].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[1].with_line, true);
        assert_eq!(
            spec.column_spec[2].alignment,
            Some(ColumnAlignment::RightJustified)
        );
        assert_eq!(spec.column_spec[2].with_line, false);
    }

    #[test]
    fn column_parse_line_at_beginning() {
        let arena = Arena::new();
        let spec = parse_column_specification("|ccc", &arena).unwrap();
        assert_eq!(spec.begins_with_line, true);
        assert_eq!(spec.column_spec.len(), 3);
        assert_eq!(
            spec.column_spec[0].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[0].with_line, false);
        assert_eq!(
            spec.column_spec[1].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[1].with_line, false);
        assert_eq!(
            spec.column_spec[2].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[2].with_line, false);
    }

    #[test]
    fn column_parse_with_spaces() {
        let arena = Arena::new();
        let spec = parse_column_specification(" c   | |   c|    | |      c ", &arena).unwrap();
        assert_eq!(spec.begins_with_line, false);
        assert_eq!(spec.column_spec.len(), 6);
        assert_eq!(
            spec.column_spec[0].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[0].with_line, true);
        assert_eq!(spec.column_spec[1].alignment, None);
        assert_eq!(spec.column_spec[1].with_line, true);
        assert_eq!(
            spec.column_spec[2].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[2].with_line, true);
        assert_eq!(spec.column_spec[3].alignment, None);
        assert_eq!(spec.column_spec[3].with_line, true);
        assert_eq!(spec.column_spec[4].alignment, None);
        assert_eq!(spec.column_spec[4].with_line, true);
        assert_eq!(
            spec.column_spec[5].alignment,
            Some(ColumnAlignment::Centered)
        );
        assert_eq!(spec.column_spec[5].with_line, false);
    }

    #[test]
    fn round_trip_em() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
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
