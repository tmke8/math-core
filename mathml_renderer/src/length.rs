//! Lengths, both absolute and relative to font size.

// use std::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;
use strum_macros::IntoStaticStr;

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LengthUnit {
    // absolute unit
    #[strum(serialize = "rem")]
    Rem,
    // relative units
    #[strum(serialize = "em")]
    Em,
    #[strum(serialize = "ex")]
    Ex,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Length {
    value: LengthValue,
    pub(crate) unit: LengthUnit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[repr(transparent)]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct LengthValue(pub(crate) f32);

impl Length {
    pub const fn new(value: f32, unit: LengthUnit) -> Self {
        Length {
            value: LengthValue(value),
            unit,
        }
    }

    pub fn push_to_string(&self, output: &mut String) {
        let mut buffer = dtoa::Buffer::new();
        let result = buffer.format(self.value.0);
        // let _ = write!(output, "{}", self.value.0).is_ok();
        output.push_str(result.strip_suffix(".0").unwrap_or(result));
        if self.value.0 != 0.0 {
            output.push_str(<&'static str>::from(self.unit));
        }
    }

    pub const fn none() -> Self {
        Length {
            value: LengthValue(f32::NAN),
            unit: LengthUnit::Rem,
        }
    }

    pub const fn zero() -> Self {
        Length {
            value: LengthValue(0.0),
            unit: LengthUnit::Rem,
        }
    }

    pub const fn into_parts(self) -> (LengthValue, LengthUnit) {
        (self.value, self.unit)
    }

    pub fn from_parts(value: LengthValue, unit: LengthUnit) -> Option<Self> {
        if value.0.is_finite() {
            Some(Length { value, unit })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LengthUnit::*;

    #[test]
    fn write() {
        let mut output = String::new();
        Length::new(0.0, Rem).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::new(1.0, Rem).push_to_string(&mut output);
        assert_eq!(&output, "1rem");
        output.clear();
        Length::new(10.0, Rem).push_to_string(&mut output);
        assert_eq!(&output, "10rem");
        output.clear();
        Length::new(5965232.0, Rem).push_to_string(&mut output);
        assert_eq!(&output, "5965232rem");
        output.clear();
        Length::new(-5965232.0, Rem).push_to_string(&mut output);
        assert_eq!(&output, "-5965232rem");
    }

    #[test]
    fn write_relative() {
        let mut output = String::new();
        Length::new(0.0, Em).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::new(0.0, Ex).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::new(1.0, Em).push_to_string(&mut output);
        assert_eq!(&output, "1em");
        output.clear();
        Length::new(1.0, Ex).push_to_string(&mut output);
        assert_eq!(&output, "1ex");
        output.clear();
        Length::new(546.0, Em).push_to_string(&mut output);
        assert_eq!(&output, "546em");
        output.clear();
        Length::new(546.0, Ex).push_to_string(&mut output);
        assert_eq!(&output, "546ex");
        output.clear();
        Length::new(-546.0, Em).push_to_string(&mut output);
        assert_eq!(&output, "-546em");
        output.clear();
        Length::new(-546.0, Ex).push_to_string(&mut output);
        assert_eq!(&output, "-546ex");
        output.clear();
    }
}
