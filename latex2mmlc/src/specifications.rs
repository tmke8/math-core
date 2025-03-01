//! Functions for parsing specifications in LaTeX commands.

use std::num::NonZeroU32;

use mathml_renderer::length::{Length, PT_IN_LEN, PX_IN_LEN};

const TEN: NonZeroU32 = NonZeroU32::new(10).unwrap();

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
    let conv = match unit {
        "pt" => PT_IN_LEN,
        "px" => PX_IN_LEN,
        _ => return Err(()),
    };
    let mut digits = digits.bytes();
    let mut acc: u32 = 0;
    let mut div = const { NonZeroU32::new(1).unwrap() };
    for digit in &mut digits {
        if digit == b'.' {
            break;
        }
        if !(b'0'..=b'9').contains(&digit) {
            return Err(());
        }
        acc *= 10;
        acc += u32::from(digit - b'0');
    }
    for digit in &mut digits {
        if !(b'0'..=b'9').contains(&digit) {
            return Err(());
        }
        acc *= 10;
        acc += u32::from(digit - b'0');
        div = div.saturating_mul(TEN);
    }
    Ok(Length::from_value((acc * conv.get() / div) as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_pt_default() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
                .push_to_string(&mut output);
            assert_eq!(s, &output);
        }
        for i in 1..10 {
            rt(&format!("{i}pt"));
            rt(&format!("0.{i}pt"));
            rt(&format!("{i}.25pt"));
            rt(&format!("{i}.75pt"));
            for j in 1..10 {
                rt(&format!("{i}{j}pt"));
                rt(&format!("{i}.{j}pt"));
            }
        }
    }

    #[test]
    fn round_trip_px() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
                .display_px(&mut output);
            assert_eq!(s, &output);
        }
        for i in 1..10 {
            rt(&format!("{i}px"));
            rt(&format!("0.{i}px"));
            rt(&format!("{i}.25px"));
            rt(&format!("{i}.75px"));
            for j in 1..10 {
                rt(&format!("{i}{j}px"));
                rt(&format!("{i}.{j}px"));
            }
        }
    }

    #[test]
    fn round_trip_pt() {
        fn rt(s: &str) {
            let mut output = String::new();
            parse_length_specification(s)
                .expect("valid")
                .display_pt(&mut output);
            assert_eq!(s, &output);
        }
        for i in 1..10 {
            rt(&format!("{i}pt"));
            rt(&format!("0.{i}pt"));
            rt(&format!("{i}.25pt"));
            rt(&format!("{i}.75pt"));
            for j in 1..10 {
                rt(&format!("{i}{j}pt"));
                rt(&format!("{i}.{j}pt"));
            }
        }
    }
}
