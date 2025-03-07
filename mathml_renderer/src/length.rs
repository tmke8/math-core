//! Lengths, both absolute and relative to font size.

use std::mem::MaybeUninit;
use std::num::{NonZeroI32, NonZeroU32};

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::itoa::fmt_u32;

pub const PT_IN_LEN: NonZeroU32 = NonZeroU32::new(360).unwrap();
pub const PX_IN_LEN: NonZeroU32 = NonZeroU32::new(480).unwrap();

pub const FONT_RELATIVE_CONV: NonZeroU32 = NonZeroU32::new(60).unwrap();

/// A specified length, either [`AbsoluteLength`] or [`FontRelativeLength`],
/// but with the bounds shaved off the top (it has about 30 significant bits).
#[derive(Debug, Clone, Copy)]
pub struct SpecifiedLength(NonZeroI32);

impl SpecifiedLength {
    /// Convert from an absolute length.
    /// This may truncate the value if it's very high or low.
    pub fn from_absolute_length(len: AbsoluteLength) -> SpecifiedLength {
        let value = len.0 << 1;
        SpecifiedLength::from_value(value)
    }

    /// Convert from a font-relative length.
    pub fn from_font_relative_length(len: FontRelativeLength) -> SpecifiedLength {
        let value = len.value << 2;
        let flag = match len.unit {
            FontRelativeUnit::Em => 0x1,
            FontRelativeUnit::Ex => 0x3,
        };
        SpecifiedLength::from_value(value | flag)
    }

    pub fn from_value(value: i32) -> SpecifiedLength {
        // The problem that we are trying to solve here is that we want a niche in `SpecifiedLength`
        // but we also want to be able to represent 0, so we cannot just use `NonZeroI32` as is.
        // What we're doing here instead is adding `i32::MIN` to the plain value, such that
        // `i32::MIN` maps to 0, where the niche is. So, we can no longer represent `i32::MIN`,
        // but that's fine. We map the original value of `i32::MIN` to 1, which corresponds to
        // `i32::MIN + 1` in the original value space.
        SpecifiedLength(
            NonZeroI32::new(value.wrapping_add(i32::MIN))
                .unwrap_or(const { NonZeroI32::new(1).unwrap() }),
        )
    }

    fn value(self) -> i32 {
        i32::from(self.0).wrapping_sub(i32::MIN)
    }

    pub fn kind(self) -> LengthKind {
        let value = self.value() >> 2;
        match self.value() & 0x3 {
            0x1 => LengthKind::FontRelativeLength(FontRelativeLength {
                value,
                unit: FontRelativeUnit::Em,
            }),
            0x3 => LengthKind::FontRelativeLength(FontRelativeLength {
                value,
                unit: FontRelativeUnit::Ex,
            }),
            _ => LengthKind::AbsoluteLength(AbsoluteLength(self.value() >> 1)),
        }
    }

    pub fn push_to_string(&self, output: &mut String) {
        match self.kind() {
            LengthKind::AbsoluteLength(len) => len.push_to_string(output),
            LengthKind::FontRelativeLength(len) => len.push_to_string(output),
        }
    }
}

impl From<AbsoluteLength> for SpecifiedLength {
    #[inline]
    fn from(a: AbsoluteLength) -> SpecifiedLength {
        SpecifiedLength::from_absolute_length(a)
    }
}

impl From<FontRelativeLength> for SpecifiedLength {
    #[inline]
    fn from(a: FontRelativeLength) -> SpecifiedLength {
        SpecifiedLength::from_font_relative_length(a)
    }
}

/// An internal wrapper type, so that we can use pattern matching
/// over both kinds of length.
#[derive(Debug, Clone, Copy)]
pub enum LengthKind {
    AbsoluteLength(AbsoluteLength),
    FontRelativeLength(FontRelativeLength),
}

/// A length, stored in $\frac{1}{360}$ of an imperial point.
///
/// This is also equal to $\frac{1}{480}$ of a CSS pixel, as demonstrated by
/// the following dimensional analysis and [CSS Values and Units 4]:
///
/// $$`
/// 1 len \times \frac{360 point}{1 nip} \times \frac{1 inch}{72 point}
/// \times \frac{96 pixel}{1 inch} = 480 pixel
/// `$$
///
/// [CSS values and units 4]:
///     https://www.w3.org/TR/css-values-4/#absolute-lengths
#[derive(Debug, Clone, Copy)]
pub struct AbsoluteLength(pub i32);

impl AbsoluteLength {
    pub fn from_pt(pt: i32) -> AbsoluteLength {
        AbsoluteLength(pt * (PT_IN_LEN.get() as i32))
    }

    pub fn from_twip(twip: i32) -> AbsoluteLength {
        AbsoluteLength(twip * (PT_IN_LEN.get() as i32 / 10) / 2)
    }

    pub fn from_px(px: i32) -> AbsoluteLength {
        AbsoluteLength(px * (PX_IN_LEN.get() as i32))
    }

    pub fn display_px(self, output: &mut String) {
        let value = self.0;
        write_impl(value, output, PX_IN_LEN, "px")
    }

    pub fn display_pt(self, output: &mut String) {
        let value = self.0;
        write_impl(value, output, PT_IN_LEN, "pt")
    }

    pub fn push_to_string(&self, output: &mut String) {
        let value = self.0;
        if value == 0 {
            output.push('0');
        } else {
            let (conv, unit) = if value % ((PT_IN_LEN.get() as i32) / 10) == 0
                || value % ((PX_IN_LEN.get() as i32) / 10) != 0
            {
                // If this measure can be serialized in 10ths of a point,
                // do that.
                (PT_IN_LEN, "pt")
            } else {
                (PX_IN_LEN, "px")
            };
            write_impl(value, output, conv, unit)
        }
    }
}

/// A font-relative length, stored with an explicit tag and the type of
/// font multiplied by 60.
#[derive(Debug, Clone, Copy)]
pub struct FontRelativeLength {
    pub unit: FontRelativeUnit,
    pub value: i32,
}

impl FontRelativeLength {
    pub fn from_ex(ex: i32) -> FontRelativeLength {
        FontRelativeLength {
            value: ex * 60,
            unit: FontRelativeUnit::Ex,
        }
    }
    pub fn from_em(em: i32) -> FontRelativeLength {
        FontRelativeLength {
            value: em * 60,
            unit: FontRelativeUnit::Em,
        }
    }
    pub fn push_to_string(&self, output: &mut String) {
        let value = self.value;
        if value == 0 {
            output.push('0');
        } else {
            write_impl(
                value,
                output,
                FONT_RELATIVE_CONV,
                match self.unit {
                    FontRelativeUnit::Em => "em",
                    FontRelativeUnit::Ex => "ex",
                },
            )
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FontRelativeUnit {
    Em = 0x1,
    Ex = 0x3,
}

fn write_impl(value: i32, output: &mut String, conv: NonZeroU32, unit: &str) {
    if value < 0 {
        output.push('-');
    }
    let value = value.unsigned_abs();
    let mut buf = [MaybeUninit::uninit(); 10];
    output.push_str(fmt_u32(value / conv, &mut buf));
    let frac = value % conv;
    if frac != 0 {
        // only write two decimal points
        output.push('.');
        output.push_str(fmt_u32(frac * 10 / conv, &mut buf));
        let frac = (frac * 10) % conv;
        if frac != 0 {
            output.push_str(fmt_u32(frac * 10 / conv, &mut buf));
        }
    }
    output.push_str(unit)
}

#[cfg(feature = "serde")]
impl Serialize for SpecifiedLength {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.kind() {
            LengthKind::AbsoluteLength(len) => len.serialize(serializer),
            LengthKind::FontRelativeLength(len) => len.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for AbsoluteLength {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.0)
    }
}

#[cfg(feature = "serde")]
impl Serialize for FontRelativeLength {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (self.value, self.unit).serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl Serialize for FontRelativeUnit {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (match *self {
            FontRelativeUnit::Em => "em",
            FontRelativeUnit::Ex => "ex",
        })
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write() {
        let mut output = String::new();
        AbsoluteLength::from_pt(0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        AbsoluteLength::from_pt(1).push_to_string(&mut output);
        assert_eq!(&output, "1pt");
        output.clear();
        AbsoluteLength::from_pt(10).push_to_string(&mut output);
        assert_eq!(&output, "10pt");
        output.clear();
        AbsoluteLength::from_pt(5965232).push_to_string(&mut output);
        assert_eq!(&output, "5965232pt");
        output.clear();
        AbsoluteLength::from_pt(-5965232).push_to_string(&mut output);
        assert_eq!(&output, "-5965232pt");
        output.clear();
        AbsoluteLength::from_px(0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        AbsoluteLength::from_px(1).push_to_string(&mut output);
        assert_eq!(&output, "1px");
        output.clear();
        AbsoluteLength::from_px(10).push_to_string(&mut output);
        assert_eq!(&output, "10px");
        output.clear();
        AbsoluteLength::from_px(4473923).push_to_string(&mut output);
        assert_eq!(&output, "4473923px");
        output.clear();
        AbsoluteLength::from_px(-4473923).push_to_string(&mut output);
        assert_eq!(&output, "-4473923px");
    }

    #[test]
    fn write_relative() {
        let mut output = String::new();
        FontRelativeLength::from_em(0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        FontRelativeLength::from_ex(0).push_to_string(&mut output);
        assert_eq!(&output, "0");
        output.clear();
        FontRelativeLength::from_em(1).push_to_string(&mut output);
        assert_eq!(&output, "1em");
        output.clear();
        FontRelativeLength::from_ex(1).push_to_string(&mut output);
        assert_eq!(&output, "1ex");
        output.clear();
        FontRelativeLength::from_em(546).push_to_string(&mut output);
        assert_eq!(&output, "546em");
        output.clear();
        FontRelativeLength::from_ex(546).push_to_string(&mut output);
        assert_eq!(&output, "546ex");
        output.clear();
        FontRelativeLength::from_em(-546).push_to_string(&mut output);
        assert_eq!(&output, "-546em");
        output.clear();
        FontRelativeLength::from_ex(-546).push_to_string(&mut output);
        assert_eq!(&output, "-546ex");
        output.clear();
    }
}
