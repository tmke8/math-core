//! A length, stored in $\frac{1}{360}$ of an imperial point.
//!
//! This is also equal to $\frac{1}{480}$ of a CSS pixel, as demonstrated by
//! the following dimensional analysis and [CSS Values and Units 4]:
//!
//! $$`
//! 1 len \times \frac{360 point}{1 nip} \times \frac{1 inch}{72 point}
//! \times \frac{96 pixel}{1 inch} = 480 pixel
//! `$$
//!
//! [CSS values and units 4]:
//!     https://www.w3.org/TR/css-values-4/#absolute-lengths

use std::mem::MaybeUninit;
use std::num::NonZeroI32;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::itoa::fmt_i32;

const PT_IN_LEN: i32 = 360;
const PX_IN_LEN: i32 = 480;

/// An absolute length.
/// See [the module][crate::length] for more information.
#[derive(Debug, Clone, Copy)]
pub struct Length(NonZeroI32);

impl Length {
    pub fn from_pt(pt: i32) -> Length {
        Length::from_value(pt * PT_IN_LEN)
    }

    pub fn from_twip(twip: i32) -> Length {
        Length::from_value(twip * (PT_IN_LEN / 10) / 2)
    }

    pub fn from_px(px: i32) -> Length {
        Length::from_value(px * PX_IN_LEN)
    }

    fn from_value(value: i32) -> Length {
        Length(NonZeroI32::new(value.wrapping_add(i32::MIN)).unwrap_or(NonZeroI32::MIN))
    }

    fn value(self) -> i32 {
        i32::from(self.0).wrapping_sub(i32::MIN)
    }

    fn write_impl(self, output: &mut String, conv: i32, unit: &str) {
        let value = self.value();
        let mut buf = [MaybeUninit::uninit(); 11];
        output.push_str(fmt_i32(value / conv, &mut buf));
        let frac = value % conv;
        if frac != 0 {
            // only write two decimal points
            output.push('.');
            output.push_str(fmt_i32(frac * 10 / conv, &mut buf));
            let frac = (frac * 10) % conv;
            if frac != 0 {
                output.push_str(fmt_i32(frac * 10 / conv, &mut buf));
            }
        }
        output.push_str(unit)
    }

    #[cfg(test)]
    fn display_px(self, output: &mut String) {
        self.write_impl(output, PX_IN_LEN, "px")
    }

    #[cfg(test)]
    fn display_pt(self, output: &mut String) {
        self.write_impl(output, PT_IN_LEN, "pt")
    }

    pub fn push_to_string(&self, output: &mut String) {
        let value = self.value();
        if value == 0 {
            output.push('0');
        } else {
            let (conv, unit) = if value % (PT_IN_LEN / 10) == 0 || value % (PX_IN_LEN / 10) != 0 {
                // If this measure can be serialized in 10ths of a point,
                // do that.
                (PT_IN_LEN, "pt")
            } else {
                (PX_IN_LEN, "px")
            };
            self.write_impl(output, conv, unit)
        }
    }
}

impl FromStr for Length {
    type Err = ();
    fn from_str(s: &str) -> Result<Length, Self::Err> {
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
        let mut acc = 0;
        let mut div = 1;
        for digit in &mut digits {
            if digit == b'.' {
                break;
            }
            if !(b'0'..=b'9').contains(&digit) {
                return Err(());
            }
            acc *= 10;
            acc += i32::from(digit - b'0');
        }
        for digit in &mut digits {
            if !(b'0'..=b'9').contains(&digit) {
                return Err(());
            }
            acc *= 10;
            acc += i32::from(digit - b'0');
            div *= 10;
        }
        Ok(Length::from_value(acc * conv / div))
    }
}

#[cfg(feature = "serde")]
impl Serialize for Length {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_pt_default() {
        fn rt(s: &str) {
            let mut output = String::new();
            Length::from_str(s)
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
            Length::from_str(s).expect("valid").display_px(&mut output);
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
            Length::from_str(s).expect("valid").display_pt(&mut output);
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
