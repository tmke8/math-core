use std::{
    fmt::{self, Write},
    iter::FusedIterator,
};

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::symbol;

/// A `SuperChar` is like a `char`, and has the same size as one,
/// but can additionally encode that the character is followed by
/// a variation selector in the range U+FE00-U+FE0E,
/// U+0338 (long solidus overlay), or U+20D2 (long vertical line overlay).
//
// # Structure
//
// - High 4 bits: 1-15 for VS1-VS15, or 0 for no variation seq
// - Bit 5 (from MSB): U+0338
// - Bit 6: U+20D2
// - Low 21 bits: the base `char`
// - Remaining 3 bits unused for now
//
// # Safety
//
// `self.0 & CHAR_MASK` must always be a valid `char`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SuperChar(u32);

/// Mask for the top 4 bits
const VS_MASK: u32 = 0xF000_0000;

/// Mask for the flag bits
const FLAGS_MASK: u32 = 0x0FE0_0000;

/// Mask for the low 21 bits
const CHAR_MASK: u32 = 0x001F_FFFF;

/// Mask for U+0338
const SOLIDUS_BIT: u32 = 0x0800_0000;
/// Mask for U+20D2
const VERTICAL_LINE_BIT: u32 = 0x0400_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum VariationSelector {
    /// `'\u{FE00}'`
    Vs1 = 1,
    /// `'\u{FE01}'`
    Vs2 = 2,
    /// `'\u{FE02}'`
    Vs3 = 3,
    /// `'\u{FE03}'`
    Vs4 = 4,
    /// `'\u{FE04}'`
    Vs5 = 5,
    /// `'\u{FE05}'`
    Vs6 = 6,
    /// `'\u{FE06}'`
    Vs7 = 7,
    /// `'\u{FE07}'`
    Vs8 = 8,
    /// `'\u{FE08}'`
    Vs9 = 9,
    /// `'\u{FE09}'`
    Vs10 = 10,
    /// `'\u{FE0A}'`
    Vs11 = 11,
    /// `'\u{FE0B}'`
    Vs12 = 12,
    /// `'\u{FE0C}'`
    Vs13 = 13,
    /// `'\u{FE0D}'`
    Vs14 = 14,
    /// `'\u{FE0E}'`
    Vs15 = 15,
}

impl From<VariationSelector> for char {
    #[inline]
    fn from(vs: VariationSelector) -> Self {
        char::from_u32(0xFDFF + vs as u32).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum OverlayChar {
    /// `'\u{0338}'`
    Solidus,
    /// `'\u{20D2}'`
    VerticalLine,
}

impl From<OverlayChar> for char {
    fn from(oc: OverlayChar) -> Self {
        match oc {
            OverlayChar::Solidus => symbol::COMBINING_LONG_SOLIDUS_OVERLAY,
            OverlayChar::VerticalLine => symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY,
        }
    }
}

impl SuperChar {
    #[must_use]
    #[inline]
    pub const fn from_char(c: char) -> Self {
        Self(c as u32)
    }

    #[must_use]
    #[inline]
    pub const fn from_char_with_vs(c: char, vs: VariationSelector) -> Self {
        Self(c as u32 | ((vs as u32) << 28))
    }

    /// Number of characters in this `SuperChar`.
    #[allow(clippy::len_without_is_empty)] // a `SuperChar` is never empty
    #[must_use]
    #[inline]
    pub const fn len(self) -> usize {
        1 + ((self.0 & FLAGS_MASK).count_ones() as usize) + self.has_vs() as usize
    }

    /// Returns an iterator over the `char`s of this `SuperChar`.
    #[must_use]
    #[inline]
    pub fn chars(self) -> SuperCharChars {
        SuperCharChars(self.0)
    }

    /// Whether this `SuperChar` has a variation selector associated
    #[must_use]
    #[inline]
    pub const fn has_vs(self) -> bool {
        self.0 & VS_MASK != 0
    }

    /// The variation selector for this `SuperChar`, if it has one.
    #[must_use]
    #[inline]
    pub const fn vs(self) -> Option<VariationSelector> {
        use VariationSelector::*;
        match self.0 >> 28 {
            0 => None,
            1 => Some(Vs1),
            2 => Some(Vs2),
            3 => Some(Vs3),
            4 => Some(Vs4),
            5 => Some(Vs5),
            6 => Some(Vs6),
            7 => Some(Vs7),
            8 => Some(Vs8),
            9 => Some(Vs9),
            10 => Some(Vs10),
            11 => Some(Vs11),
            12 => Some(Vs12),
            13 => Some(Vs13),
            14 => Some(Vs14),
            15 => Some(Vs15),
            _ => unreachable!(),
        }
    }

    /// Adds the specified overlay character to this `SuperChar`, returning it as a new `SuperChar`.
    /// Idempotent if the character is already present.
    /// For the solidus overlay U+0338, we will use the precomposed form if possible.
    #[inline]
    #[must_use]
    pub fn with_overlay(self, overlay: OverlayChar) -> Self {
        match overlay {
            OverlayChar::Solidus => {
                if let Some(precomposed) = get_precomposed_solidus_overlay(self.base_char()) {
                    self.with_base_char(precomposed)
                } else {
                    Self(self.0 | SOLIDUS_BIT)
                }
            }
            OverlayChar::VerticalLine => Self(self.0 | VERTICAL_LINE_BIT),
        }
    }

    /// Get the base `char` of this `SuperChar`, disregarding variation sequences and overlays.
    #[must_use]
    #[inline]
    pub const fn base_char(self) -> char {
        // SAFETY: `self.0` field invariant
        unsafe { char::from_u32_unchecked(self.0 & CHAR_MASK) }
    }

    /// Return a version of this `SuperChar` with a different base `char`,
    /// but unchanged variation sequences and overlays.
    #[must_use]
    #[inline]
    pub const fn with_base_char(self, new_base: char) -> Self {
        Self(self.0 & !CHAR_MASK | new_base as u32)
    }

    /// If this string contains exactly 1 `char`, return it;
    /// otherwise, return `None`.
    #[must_use]
    #[inline]
    pub const fn try_as_char(self) -> Option<char> {
        if self.0 & CHAR_MASK == self.0 {
            // SAFETY: `self.0` field invariant
            Some(self.base_char())
        } else {
            None
        }
    }

    /// For now, a buffer of length 13 is sufficient to encode any `SuperChar`.
    pub fn encode_utf8(self, dst: &mut [u8]) -> &mut str {
        let mut idx: usize = 0;
        for c in self.chars() {
            let result = c.encode_utf8(&mut dst[idx..]);
            idx += result.len();
        }
        // SAFETY: we encoded valid UTF-8 into this range just above
        unsafe { str::from_utf8_unchecked_mut(&mut dst[..idx]) }
    }
}

impl From<char> for SuperChar {
    #[inline]
    fn from(c: char) -> Self {
        Self::from_char(c)
    }
}

impl fmt::Debug for SuperChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        for c in self.chars() {
            write!(f, "{}", c.escape_debug())?;
        }
        write!(f, "\"")?;
        Ok(())
    }
}

impl fmt::Display for SuperChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.chars() {
            f.write_char(c)?;
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl Serialize for SuperChar {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serializer::collect_str(serializer, self)
    }
}

/// An iterator over the chars of a [`SuperChar`].
// Invariant: `.0` field either reperesents a valid `SuperChar`,
// except that character bits may be set to all 1s to represent
// the base character having been already yielded.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct SuperCharChars(u32);

impl Iterator for SuperCharChars {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let base_char = self.0 & CHAR_MASK;
        if base_char != CHAR_MASK {
            self.0 |= CHAR_MASK;
            // SAFETY: `self.0` field invariant,
            // and we checked for `CHAR_MASK` above
            Some(unsafe { char::from_u32_unchecked(base_char) })
        } else if let Some(vs) = SuperChar(self.0).vs() {
            self.0 &= !VS_MASK;
            Some(vs.into())
        } else if self.0 & SOLIDUS_BIT != 0 {
            self.0 &= !SOLIDUS_BIT;
            Some(symbol::COMBINING_LONG_SOLIDUS_OVERLAY)
        } else if self.0 & VERTICAL_LINE_BIT != 0 {
            self.0 &= !VERTICAL_LINE_BIT;
            Some(symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY)
        } else {
            None
        }
    }
}

impl FusedIterator for SuperCharChars {}

#[inline]
fn get_precomposed_solidus_overlay(c: char) -> Option<char> {
    if let Ok(bmp) = u16::try_from(c) {
        for &SolidusPair { base, composed } in PRECOMPOSED_SOLIDUS_OVERLAY {
            if bmp == base || bmp == composed {
                // SAFETY: all entries in `PRECOMPOSED_SOLIDUS_OVERLAY` are valid `char`s
                return Some(unsafe { char::from_u32_unchecked(composed as u32) });
            }
        }
        None
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(align(4))]
struct SolidusPair {
    base: u16,
    composed: u16,
}

// This is a mapping from Unicode codepoints to their precomposed solidus-overlay variant.
// They are all in the BMP, so we use `u16` instead of `char` to save space.
// <https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BDecomposition_Mapping%3D%2F.%CC%B8%2F%7D%26%5Cp%7BisNFC%7D>
static PRECOMPOSED_SOLIDUS_OVERLAY: &[SolidusPair] = &[
    SolidusPair {
        base: 0x2190,
        composed: 0x219A,
    }, // LEFTWARDS ARROW -> LEFTWARDS ARROW WITH STROKE
    SolidusPair {
        base: 0x2192,
        composed: 0x219B,
    }, // RIGHTWARDS ARROW -> RIGHTWARDS ARROW WITH STROKE
    SolidusPair {
        base: 0x2194,
        composed: 0x21AE,
    }, // LEFT RIGHT ARROW -> LEFT RIGHT ARROW WITH STROKE
    SolidusPair {
        base: 0x21D0,
        composed: 0x21CD,
    }, // LEFTWARDS DOUBLE ARROW -> LEFTWARDS DOUBLE ARROW WITH STROKE
    SolidusPair {
        base: 0x21D4,
        composed: 0x21CE,
    }, // LEFT RIGHT DOUBLE ARROW -> LEFT RIGHT DOUBLE ARROW WITH STROKE
    SolidusPair {
        base: 0x21D2,
        composed: 0x21CF,
    }, // RIGHTWARDS DOUBLE ARROW -> RIGHTWARDS DOUBLE ARROW WITH STROKE
    SolidusPair {
        base: 0x2203,
        composed: 0x2204,
    }, // THERE EXISTS -> THERE DOES NOT EXIST
    SolidusPair {
        base: 0x2208,
        composed: 0x2209,
    }, // ELEMENT OF -> NOT AN ELEMENT OF
    SolidusPair {
        base: 0x220B,
        composed: 0x220C,
    }, // CONTAINS AS MEMBER -> DOES NOT CONTAIN AS MEMBER
    SolidusPair {
        base: 0x2223,
        composed: 0x2224,
    }, // DIVIDES -> DOES NOT DIVIDE
    SolidusPair {
        base: 0x2225,
        composed: 0x2226,
    }, // PARALLEL TO -> NOT PARALLEL TO
    SolidusPair {
        base: 0x223C,
        composed: 0x2241,
    }, // TILDE OPERATOR -> NOT TILDE
    SolidusPair {
        base: 0x2243,
        composed: 0x2244,
    }, // ASYMPTOTICALLY EQUAL TO -> NOT ASYMPTOTICALLY EQUAL TO
    SolidusPair {
        base: 0x2245,
        composed: 0x2247,
    }, // APPROXIMATELY EQUAL TO -> NEITHER APPROXIMATELY NOR ACTUALLY EQUAL TO
    SolidusPair {
        base: 0x2248,
        composed: 0x2249,
    }, // ALMOST EQUAL TO -> NOT ALMOST EQUAL TO
    SolidusPair {
        base: 0x003D,
        composed: 0x2260,
    }, // EQUALS SIGN -> NOT EQUAL TO
    SolidusPair {
        base: 0x2261,
        composed: 0x2262,
    }, // IDENTICAL TO -> NOT IDENTICAL TO
    SolidusPair {
        base: 0x224D,
        composed: 0x226D,
    }, // EQUIVALENT TO -> NOT EQUIVALENT TO
    SolidusPair {
        base: 0x003C,
        composed: 0x226E,
    }, // LESS-THAN SIGN -> NOT LESS-THAN
    SolidusPair {
        base: 0x003E,
        composed: 0x226F,
    }, // GREATER-THAN SIGN -> NOT GREATER-THAN
    SolidusPair {
        base: 0x2264,
        composed: 0x2270,
    }, // LESS-THAN OR EQUAL TO -> NEITHER LESS-THAN NOR EQUAL TO
    SolidusPair {
        base: 0x2265,
        composed: 0x2271,
    }, // GREATER-THAN OR EQUAL TO -> NEITHER GREATER-THAN NOR EQUAL TO
    SolidusPair {
        base: 0x2272,
        composed: 0x2274,
    }, // LESS-THAN OR EQUIVALENT TO -> NEITHER LESS-THAN NOR EQUIVALENT TO
    SolidusPair {
        base: 0x2273,
        composed: 0x2275,
    }, // GREATER-THAN OR EQUIVALENT TO -> NEITHER GREATER-THAN NOR EQUIVALENT TO
    SolidusPair {
        base: 0x2276,
        composed: 0x2278,
    }, // LESS-THAN OR GREATER-THAN -> NEITHER LESS-THAN NOR GREATER-THAN
    SolidusPair {
        base: 0x2277,
        composed: 0x2279,
    }, // GREATER-THAN OR LESS-THAN -> NEITHER GREATER-THAN NOR LESS-THAN
    SolidusPair {
        base: 0x227A,
        composed: 0x2280,
    }, // PRECEDES -> DOES NOT PRECEDE
    SolidusPair {
        base: 0x227B,
        composed: 0x2281,
    }, // SUCCEEDS -> DOES NOT SUCCEED
    SolidusPair {
        base: 0x2282,
        composed: 0x2284,
    }, // SUBSET OF -> NOT A SUBSET OF
    SolidusPair {
        base: 0x2283,
        composed: 0x2285,
    }, // SUPERSET OF -> NOT A SUPERSET OF
    SolidusPair {
        base: 0x2286,
        composed: 0x2288,
    }, // SUBSET OF OR EQUAL TO -> NEITHER A SUBSET OF NOR EQUAL TO
    SolidusPair {
        base: 0x2287,
        composed: 0x2289,
    }, // SUPERSET OF OR EQUAL TO -> NEITHER A SUPERSET OF NOR EQUAL TO
    SolidusPair {
        base: 0x22A2,
        composed: 0x22AC,
    }, // RIGHT TACK -> DOES NOT PROVE
    SolidusPair {
        base: 0x22A8,
        composed: 0x22AD,
    }, // TRUE -> NOT TRUE
    SolidusPair {
        base: 0x22A9,
        composed: 0x22AE,
    }, // FORCES -> DOES NOT FORCE
    SolidusPair {
        base: 0x22AB,
        composed: 0x22AF,
    }, // DOUBLE VERTICAL BAR DOUBLE RIGHT TURNSTILE -> NEGATED DOUBLE VERTICAL BAR DOUBLE RIGHT TURNSTILE
    SolidusPair {
        base: 0x227C,
        composed: 0x22E0,
    }, // PRECEDES OR EQUAL TO -> DOES NOT PRECEDE OR EQUAL
    SolidusPair {
        base: 0x227D,
        composed: 0x22E1,
    }, // SUCCEEDS OR EQUAL TO -> DOES NOT SUCCEED OR EQUAL
    SolidusPair {
        base: 0x2291,
        composed: 0x22E2,
    }, // SQUARE IMAGE OF OR EQUAL TO -> NOT SQUARE IMAGE OF OR EQUAL TO
    SolidusPair {
        base: 0x2292,
        composed: 0x22E3,
    }, // SQUARE ORIGINAL OF OR EQUAL TO -> NOT SQUARE ORIGINAL OF OR EQUAL TO
    SolidusPair {
        base: 0x22B2,
        composed: 0x22EA,
    }, // NORMAL SUBGROUP OF -> NOT NORMAL SUBGROUP OF
    SolidusPair {
        base: 0x22B3,
        composed: 0x22EB,
    }, // CONTAINS AS NORMAL SUBGROUP -> DOES NOT CONTAIN AS NORMAL SUBGROUP
    SolidusPair {
        base: 0x22B4,
        composed: 0x22EC,
    }, // NORMAL SUBGROUP OF OR EQUAL TO -> NOT NORMAL SUBGROUP OF OR EQUAL TO
    SolidusPair {
        base: 0x22B5,
        composed: 0x22ED,
    }, // CONTAINS AS NORMAL SUBGROUP OR EQUAL TO -> DOES NOT CONTAIN AS NORMAL SUBGROUP OR EQUAL
];
