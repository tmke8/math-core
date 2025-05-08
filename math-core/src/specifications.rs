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

    let value = simple_float_parse(digits)?;
    // let value = digits.parse::<f32>().map_err(|_| ())?;

    match unit {
        "pt" => Ok(Length::from_pt(value)),
        "em" => Ok(Length::from_em(value)),
        "ex" => Ok(Length::from_ex(value)),
        _ => return Err(()),
    }
}

/// Simple float parsing.
///
/// This is much less sophisticated than `digits.parse::<f32>()` but it has the advantage that it
/// produces very small code.
///
/// The largest number this function can handle is 4294967295.4294967295 and the smallest is the
/// same but with a minus sign.
fn simple_float_parse(digits: &str) -> Result<f32, ()> {
    let (digits, sign) = if let Some(digits) = digits.strip_prefix('-') {
        (digits, -1.0f32)
    } else {
        (digits, 1.0f32)
    };
    let (integer, fraction) = if let Some(parts) = digits.split_once('.') {
        parts
    } else {
        (digits, "")
    };
    let integer = integer.parse::<u32>().map_err(|_| ())?;
    let frac_len = fraction.len() as u32;
    let mut value = if frac_len > 0 {
        let fraction = fraction.parse::<u32>().map_err(|_| ())?;
        to_nearest_f32(integer, fraction, frac_len)
    } else {
        integer as f32
    };
    value *= sign;
    Ok(value)
}
fn to_nearest_f32(integer: u32, fraction: u32, frac_len: u32) -> f32 {
    // Calculate the denominator (power of 10) for the fraction
    let denominator = 10u128.pow(frac_len);

    // Convert to a single rational number (numerator/denominator)
    let numerator = (integer as u128) * denominator + (fraction as u128);

    // Convert the rational number to f32
    ((numerator as f64) / (denominator as f64)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_floats() {
        assert!(simple_float_parse("1..0").is_err());
        assert!(simple_float_parse("--1").is_err());
        assert!(simple_float_parse("nan").is_err());
        assert!(simple_float_parse("4294967296.0").is_err());
    }

    #[test]
    fn test_simple_float_parse() {
        assert_eq!(simple_float_parse("1.0").unwrap(), 1.0);
        assert_eq!(simple_float_parse("4294967295.0").unwrap(), 4294967300.0);
        assert_eq!(
            simple_float_parse("-4294967295.4294967295").unwrap(),
            -4294967300.0
        );
        assert_eq!(simple_float_parse("0.4294967295").unwrap(), 0.42949674);
        assert_eq!(simple_float_parse("16777216.0").unwrap(), 16777216.0);
        assert_eq!(simple_float_parse("16777217.0").unwrap(), 16777216.0);
        assert_eq!(simple_float_parse("16777218.0").unwrap(), 16777218.0);
        assert_eq!(simple_float_parse("16777219.0").unwrap(), 16777220.0);
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
