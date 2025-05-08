//! Functions for parsing specifications in LaTeX commands.

use mathml_renderer::length::Length;

pub(crate) fn parse_length_specification(s: &str) -> Result<Length, ()> {
    let len = s.len();
    // We need at least 2 characters to have a unit.
    let Some(unit_offset) = len.checked_sub(2) else {
        return Err(());
    };
    // Check whether we can split the string at the unit offset.
    // (This can fail if `unit_offset` is not a valid UTF-8 boundary.)
    let Some((digits, unit)) = s.split_at_checked(unit_offset) else {
        return Err(());
    };
    let value = digits.parse::<f32>().map_err(|_| ())?;
    // let value = lexical_core::parse::<f32>(digits.as_bytes()).map_err(|_| ())?;
    match unit {
        "pt" => Ok(Length::from_pt(value)),
        "em" => Ok(Length::from_em(value)),
        "ex" => Ok(Length::from_ex(value)),
        _ => return Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
