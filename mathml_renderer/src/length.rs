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

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::Serialize;

const PT_IN_LEN: i32 = 360;
const PX_IN_LEN: i32 = 480;

/// An absolute length.
/// See [the module][crate::length] for more information.
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Length(pub i32);

impl Length {
    pub fn from_pt(pt: i32) -> Length {
        Length(pt * PT_IN_LEN)
    }
    pub fn from_twip(twip: i32) -> Length {
        Length(twip * (PT_IN_LEN / 10) / 2)
    }
    pub fn from_px(px: i32) -> Length {
        Length(px * PX_IN_LEN)
    }

    fn write_impl(self, f: &mut Formatter<'_>, conv: i32, unit: &str) -> fmt::Result {
        <i32 as Display>::fmt(&(self.0 / conv), f)?;
        let frac = self.0 % conv;
        if frac != 0 {
            // only write two decimal points
            f.write_str(".")?;
            <i32 as Display>::fmt(&(frac * 10 / conv), f)?;
            let frac = (frac * 10) % conv;
            if frac != 0 {
                <i32 as Display>::fmt(&(frac * 10 / conv), f)?;
            }
        }
        f.write_str(unit)
    }

    pub fn display_px(self) -> impl fmt::Display {
        struct Wrap(Length);
        impl Display for Wrap {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.write_impl(f, PX_IN_LEN, "px")
            }
        }
        Wrap(self)
    }

    pub fn display_pt(self) -> impl fmt::Display {
        struct Wrap(Length);
        impl Display for Wrap {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.write_impl(f, PT_IN_LEN, "pt")
            }
        }
        Wrap(self)
    }
}

impl FromStr for Length {
    type Err = LengthParseError;
    fn from_str(s: &str) -> Result<Length, LengthParseError> {
        let conv = if s.ends_with("pt") {
            PT_IN_LEN
        } else if s.ends_with("px") {
            PX_IN_LEN
        } else {
            return Err(LengthParseError);
        };
        let mut digits = s[..s.len() - 2].bytes();
        let mut acc = 0;
        let mut div = 1;
        for digit in &mut digits {
            if digit == b'.' {
                break;
            }
            if digit < b'0' || digit > b'9' {
                return Err(LengthParseError);
            }
            acc *= 10;
            acc += i32::from(digit - b'0');
        }
        for digit in &mut digits {
            if digit < b'0' || digit > b'9' {
                return Err(LengthParseError);
            }
            acc *= 10;
            acc += i32::from(digit - b'0');
            div *= 10;
        }
        Ok(Length(acc * conv / div))
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            f.write_str("0")
        } else {
            let (conv, unit) = if self.0 % (PT_IN_LEN / 10) == 0 || self.0 % (PX_IN_LEN / 10) != 0 {
                // If this measure can be serialized in 10ths of a point,
                // do that.
                (PT_IN_LEN, "pt")
            } else {
                (PX_IN_LEN, "px")
            };
            self.write_impl(f, conv, unit)
        }
    }
}
impl Debug for Length {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Length(")?;
        <Self as Display>::fmt(self, f)?;
        f.write_str(")")
    }
}

/// A failed conversion from decimal to `Length`.
pub struct LengthParseError;

impl Error for LengthParseError {}
impl Display for LengthParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Length parse error")
    }
}
impl Debug for LengthParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("LengthParseError")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_pt_default() {
        fn rt(s: &str) {
            assert_eq!(s, &Length::from_str(s).expect("valid").to_string());
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
            assert_eq!(s, &Length::from_str(s).expect("valid").display_px().to_string());
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
            assert_eq!(s, &Length::from_str(s).expect("valid").display_pt().to_string());
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

