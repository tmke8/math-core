//! Lengths, both absolute and relative to font size.

// use std::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;
use strum_macros::AsRefStr;

#[derive(Debug, Clone, Copy, AsRefStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LengthUnit {
    // absolute unit
    #[strum(serialize = "px")]
    Px,
    // relative units
    #[strum(serialize = "em")]
    Em, // (mu is converted to this)
    #[strum(serialize = "ex")]
    Ex,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Length {
    value: LengthValue,
    unit: LengthUnit,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[repr(transparent)]
pub struct LengthValue(pub(crate) f32);

impl Length {
    pub fn push_to_string(&self, output: &mut String) {
        let mut buffer = dtoa::Buffer::new();
        let result = buffer.format(self.value.0);
        // let _ = write!(output, "{}", self.value.0).is_ok();
        output.push_str(result.strip_suffix(".0").unwrap_or(result));
        if self.value.0 != 0.0 {
            output.push_str(self.unit.as_ref());
        }
    }

    pub const fn empty() -> Self {
        Length {
            value: LengthValue(f32::NAN),
            unit: LengthUnit::Px,
        }
    }

    pub const fn zero() -> Self {
        Length {
            value: LengthValue(0.0),
            unit: LengthUnit::Px,
        }
    }

    pub fn from_mu(mu: f32) -> Self {
        Length {
            value: LengthValue(((mu as f64) * 0.05555555555) as f32),
            unit: LengthUnit::Em,
        }
    }

    pub fn from_pt(pt: f32) -> Self {
        Length {
            value: LengthValue(pt * 0.1),
            unit: LengthUnit::Em,
        }
    }

    #[inline(always)]
    pub const fn from_px(px: f32) -> Self {
        Length {
            value: LengthValue(px),
            unit: LengthUnit::Px,
        }
    }

    #[inline(always)]
    pub const fn from_ex(ex: f32) -> Self {
        Length {
            value: LengthValue(ex),
            unit: LengthUnit::Ex,
        }
    }

    #[inline(always)]
    pub const fn from_em(em: f32) -> Self {
        Length {
            value: LengthValue(em),
            unit: LengthUnit::Em,
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

    #[test]
    fn write() {
        let mut output = String::new();
        Length::from_pt(0.0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::from_pt(1.0).push_to_string(&mut output);
        assert_eq!(&output, "0.1em");
        output.clear();
        Length::from_pt(10.0).push_to_string(&mut output);
        assert_eq!(&output, "1em");
        output.clear();
        Length::from_pt(5965232.0).push_to_string(&mut output);
        assert_eq!(&output, "596523.2em");
        output.clear();
        Length::from_pt(-5965232.0).push_to_string(&mut output);
        assert_eq!(&output, "-596523.2em");
        output.clear();
        Length::from_px(0.0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::from_px(1.0).push_to_string(&mut output);
        assert_eq!(&output, "1px");
        output.clear();
        Length::from_px(10.0).push_to_string(&mut output);
        assert_eq!(&output, "10px");
        output.clear();
        Length::from_px(4473923.0).push_to_string(&mut output);
        assert_eq!(&output, "4473923px");
        output.clear();
        Length::from_px(-4473923.0).push_to_string(&mut output);
        assert_eq!(&output, "-4473923px");
    }

    #[test]
    fn write_relative() {
        let mut output = String::new();
        Length::from_em(0.0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::from_ex(0.0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        Length::from_em(1.0).push_to_string(&mut output);
        assert_eq!(&output, "1em");
        output.clear();
        Length::from_ex(1.0).push_to_string(&mut output);
        assert_eq!(&output, "1ex");
        output.clear();
        Length::from_em(546.0).push_to_string(&mut output);
        assert_eq!(&output, "546em");
        output.clear();
        Length::from_ex(546.0).push_to_string(&mut output);
        assert_eq!(&output, "546ex");
        output.clear();
        Length::from_em(-546.0).push_to_string(&mut output);
        assert_eq!(&output, "-546em");
        output.clear();
        Length::from_ex(-546.0).push_to_string(&mut output);
        assert_eq!(&output, "-546ex");
        output.clear();
    }
}
