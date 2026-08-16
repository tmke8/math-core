//! Lengths, both absolute and relative to font size.

use alloc::string::String;

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

    pub const fn is_negative(self) -> bool {
        self.value.0 < 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct LengthSet {
    pub rem: LengthValue,
    pub em: LengthValue,
    pub ex: LengthValue,
}

impl LengthSet {
    pub fn zero() -> LengthSet {
        LengthSet {
            rem: LengthValue(0.0),
            em: LengthValue(0.0),
            ex: LengthValue(0.0),
        }
    }
    pub fn iter(self) -> impl Iterator<Item = Length> {
        struct Iter(LengthSet);
        impl Iterator for Iter {
            type Item = Length;
            fn next(&mut self) -> Option<Length> {
                if self.0.rem.0 != 0.0 {
                    let rem = Length::from_parts(self.0.rem, LengthUnit::Rem);
                    self.0.rem.0 = 0.0;
                    rem
                } else if self.0.em.0 != 0.0 {
                    let em = Length::from_parts(self.0.em, LengthUnit::Em);
                    self.0.em.0 = 0.0;
                    em
                } else if self.0.ex.0 != 0.0 {
                    let ex = Length::from_parts(self.0.ex, LengthUnit::Ex);
                    self.0.ex.0 = 0.0;
                    ex
                } else {
                    None
                }
            }
        }
        Iter(self)
    }
}

impl From<Length> for LengthSet {
    fn from(value: Length) -> Self {
        LengthSet::zero() + value
    }
}

impl core::ops::AddAssign for LengthSet {
    fn add_assign(&mut self, rhs: Self) {
        *self = LengthSet {
            rem: LengthValue(self.rem.0 + rhs.rem.0),
            em: LengthValue(self.em.0 + rhs.em.0),
            ex: LengthValue(self.ex.0 + rhs.ex.0),
        };
    }
}

impl core::ops::Add<LengthSet> for LengthSet {
    type Output = LengthSet;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl core::ops::Add<Length> for LengthSet {
    type Output = LengthSet;

    fn add(mut self, rhs: Length) -> Self::Output {
        match rhs.into_parts() {
            (value, LengthUnit::Rem) => self.rem.0 += value.0,
            (value, LengthUnit::Em) => self.em.0 += value.0,
            (value, LengthUnit::Ex) => self.ex.0 += value.0,
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LengthUnit::*;

    #[test]
    fn test_write() {
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
    fn test_write_relative() {
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
