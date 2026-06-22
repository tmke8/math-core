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
//
// # Safety
//
// `self.0 & CHAR_MASK` must always be a valid `char`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SuperChar(u32);

/// Mask for the top 4 bits
const VS_MASK: u32 = 0xF000_0000;

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
    pub const MAX_LEN_UTF8: usize = 12;

    #[must_use]
    #[inline]
    pub const fn from_char(c: char) -> Self {
        Self(c as u32)
    }

    /// Characters with canonical decompositions should not take a VS.
    /// We `debug_assert!` for this with respect to the solidus overlay U+0338.
    /// (There's no unsoundenss if it happens, it's just not correct Unicode usage)
    #[must_use]
    #[inline]
    pub const fn from_char_with_vs(c: char, vs: VariationSelector) -> Self {
        debug_assert!(!is_precomposed_solidus_overlay_for_debug(c));
        Self(c as u32 | ((vs as u32) << 28))
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
    /// For the solidus overlay U+0338, we will use the precomposed form if one exists and
    /// there is no variation selector set.
    #[inline]
    #[must_use]
    pub fn with_overlay(self, overlay: OverlayChar) -> Self {
        match overlay {
            OverlayChar::Solidus => {
                if !self.has_vs()
                    && let Some(precomposed) = get_precomposed_solidus_overlay(self.base_char())
                {
                    // swap out base char for precomposed form
                    Self(self.0 & !CHAR_MASK | precomposed as u32)
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

    /// See [`Self::MAX_LEN_UTF8`] for the number of bytes needed.
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
        PRECOMPOSED_SOLIDUS_OVERLAY
            .0
            .binary_search_by_key(&bmp, |&[from, _]| from)
            .ok()
            // SAFETY: `PRECOMPOSED_SOLIDUS_OVERLAY` contains only valid Unicode characters
            .map(|idx| unsafe {
                char::from_u32_unchecked(PRECOMPOSED_SOLIDUS_OVERLAY.0[idx][1].into())
            })
    } else {
        None
    }
}

/// Whether this character has a canonical decomposition to a sequence containing U+338.
/// Used in a debug assertion; should not be used in release because it is slow
const fn is_precomposed_solidus_overlay_for_debug(c: char) -> bool {
    // can't use `binary_search` in `const`,
    // so we do a linear one

    let cp = c as u32;
    if cp > u16::MAX as u32 {
        return false;
    }
    let cp = cp as u16;

    let mut i: usize = 0;
    while i < PRECOMPOSED_SOLIDUS_OVERLAY.0.len() {
        if cp == PRECOMPOSED_SOLIDUS_OVERLAY.0[i][1] {
            return true;
        }
        i += 1;
    }

    false
}

#[repr(align(128))] // align to cache line
struct Align128<T>(T);

// This is a mapping from Unicode codepoints to their precomposed solidus-overlay variant.
// They are all in the BMP, so we use `u16` instead of `char` to save space.
// <https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BDecomposition_Mapping%3D%2F.%CC%B8%2F%7D%26%5Cp%7BisNFC%7D>
const PRECOMPOSED_SOLIDUS_OVERLAY: Align128<[[u16; 2]; 88]> = Align128([
    [0x003C, 0x226E], // LESS-THAN SIGN -> NOT LESS-THAN
    [0x003D, 0x2260], // EQUALS SIGN -> NOT EQUAL TO
    [0x003E, 0x226F], // GREATER-THAN SIGN -> NOT GREATER-THAN
    [0x2190, 0x219A], // LEFTWARDS ARROW -> LEFTWARDS ARROW WITH STROKE
    [0x2192, 0x219B], // RIGHTWARDS ARROW -> RIGHTWARDS ARROW WITH STROKE
    [0x2194, 0x21AE], // LEFT RIGHT ARROW -> LEFT RIGHT ARROW WITH STROKE
    [0x219A, 0x219A], // LEFTWARDS ARROW WITH STROKE
    [0x219B, 0x219B], // RIGHTWARDS ARROW WITH STROKE
    [0x21AE, 0x21AE], // LEFT RIGHT ARROW WITH STROKE
    [0x21CD, 0x21CD], // LEFTWARDS DOUBLE ARROW WITH STROKE
    [0x21CE, 0x21CE], // LEFT RIGHT DOUBLE ARROW WITH STROKE
    [0x21CF, 0x21CF], // RIGHTWARDS DOUBLE ARROW WITH STROKE
    [0x21D0, 0x21CD], // LEFTWARDS DOUBLE ARROW -> LEFTWARDS DOUBLE ARROW WITH STROKE
    [0x21D2, 0x21CF], // RIGHTWARDS DOUBLE ARROW -> RIGHTWARDS DOUBLE ARROW WITH STROKE
    [0x21D4, 0x21CE], // LEFT RIGHT DOUBLE ARROW -> LEFT RIGHT DOUBLE ARROW WITH STROKE
    [0x2203, 0x2204], // THERE EXISTS -> THERE DOES NOT EXIST
    [0x2204, 0x2204], // THERE DOES NOT EXIST
    [0x2208, 0x2209], // ELEMENT OF -> NOT AN ELEMENT OF
    [0x2209, 0x2209], // NOT AN ELEMENT OF
    [0x220B, 0x220C], // CONTAINS AS MEMBER -> DOES NOT CONTAIN AS MEMBER
    [0x220C, 0x220C], // DOES NOT CONTAIN AS MEMBER
    [0x2223, 0x2224], // DIVIDES -> DOES NOT DIVIDE
    [0x2224, 0x2224], // DOES NOT DIVIDE
    [0x2225, 0x2226], // PARALLEL TO -> NOT PARALLEL TO
    [0x2226, 0x2226], // NOT PARALLEL TO
    [0x223C, 0x2241], // TILDE OPERATOR -> NOT TILDE
    [0x2241, 0x2241], // NOT TILDE
    [0x2243, 0x2244], // ASYMPTOTICALLY EQUAL TO -> NOT ASYMPTOTICALLY EQUAL TO
    [0x2244, 0x2244], // NOT ASYMPTOTICALLY EQUAL TO
    [0x2245, 0x2247], // APPROXIMATELY EQUAL TO -> NEITHER APPROXIMATELY NOR ACTUALLY EQUAL TO
    [0x2247, 0x2247], // NEITHER APPROXIMATELY NOR ACTUALLY EQUAL TO
    [0x2248, 0x2249], // ALMOST EQUAL TO -> NOT ALMOST EQUAL TO
    [0x2249, 0x2249], // NOT ALMOST EQUAL TO
    [0x224D, 0x226D], // EQUIVALENT TO -> NOT EQUIVALENT TO
    [0x2260, 0x2260], // NOT EQUAL TO
    [0x2261, 0x2262], // IDENTICAL TO -> NOT IDENTICAL TO
    [0x2262, 0x2262], // NOT IDENTICAL TO
    [0x2264, 0x2270], // LESS-THAN OR EQUAL TO -> NEITHER LESS-THAN NOR EQUAL TO
    [0x2265, 0x2271], // GREATER-THAN OR EQUAL TO -> NEITHER GREATER-THAN NOR EQUAL TO
    [0x226D, 0x226D], // NOT EQUIVALENT TO
    [0x226E, 0x226E], // NOT LESS-THAN
    [0x226F, 0x226F], // NOT GREATER-THAN
    [0x2270, 0x2270], // NEITHER LESS-THAN NOR EQUAL TO
    [0x2271, 0x2271], // NEITHER GREATER-THAN NOR EQUAL TO
    [0x2272, 0x2274], // LESS-THAN OR EQUIVALENT TO -> NEITHER LESS-THAN NOR EQUIVALENT TO
    [0x2273, 0x2275], // GREATER-THAN OR EQUIVALENT TO -> NEITHER GREATER-THAN NOR EQUIVALENT TO
    [0x2274, 0x2274], // NEITHER LESS-THAN NOR EQUIVALENT TO
    [0x2275, 0x2275], // NEITHER GREATER-THAN NOR EQUIVALENT TO
    [0x2276, 0x2278], // LESS-THAN OR GREATER-THAN -> NEITHER LESS-THAN NOR GREATER-THAN
    [0x2277, 0x2279], // GREATER-THAN OR LESS-THAN -> NEITHER GREATER-THAN NOR LESS-THAN
    [0x2278, 0x2278], // NEITHER LESS-THAN NOR GREATER-THAN
    [0x2279, 0x2279], // NEITHER GREATER-THAN NOR LESS-THAN
    [0x227A, 0x2280], // PRECEDES -> DOES NOT PRECEDE
    [0x227B, 0x2281], // SUCCEEDS -> DOES NOT SUCCEED
    [0x227C, 0x22E0], // PRECEDES OR EQUAL TO -> DOES NOT PRECEDE OR EQUAL
    [0x227D, 0x22E1], // SUCCEEDS OR EQUAL TO -> DOES NOT SUCCEED OR EQUAL
    [0x2280, 0x2280], // DOES NOT PRECEDE
    [0x2281, 0x2281], // DOES NOT SUCCEED
    [0x2282, 0x2284], // SUBSET OF -> NOT A SUBSET OF
    [0x2283, 0x2285], // SUPERSET OF -> NOT A SUPERSET OF
    [0x2284, 0x2284], // NOT A SUBSET OF
    [0x2285, 0x2285], // NOT A SUPERSET OF
    [0x2286, 0x2288], // SUBSET OF OR EQUAL TO -> NEITHER A SUBSET OF NOR EQUAL TO
    [0x2287, 0x2289], // SUPERSET OF OR EQUAL TO -> NEITHER A SUPERSET OF NOR EQUAL TO
    [0x2288, 0x2288], // NEITHER A SUBSET OF NOR EQUAL TO
    [0x2289, 0x2289], // NEITHER A SUPERSET OF NOR EQUAL TO
    [0x2291, 0x22E2], // SQUARE IMAGE OF OR EQUAL TO -> NOT SQUARE IMAGE OF OR EQUAL TO
    [0x2292, 0x22E3], // SQUARE ORIGINAL OF OR EQUAL TO -> NOT SQUARE ORIGINAL OF OR EQUAL TO
    [0x22A2, 0x22AC], // RIGHT TACK -> DOES NOT PROVE
    [0x22A8, 0x22AD], // TRUE -> NOT TRUE
    [0x22A9, 0x22AE], // FORCES -> DOES NOT FORCE
    [0x22AB, 0x22AF], // DOUBLE VERTICAL BAR DOUBLE RIGHT TURNSTILE -> NEGATED DOUBLE VERTICAL BAR DOUBLE RIGHT TURNSTILE
    [0x22AC, 0x22AC], // DOES NOT PROVE
    [0x22AD, 0x22AD], // NOT TRUE
    [0x22AE, 0x22AE], // DOES NOT FORCE
    [0x22AF, 0x22AF], // NEGATED DOUBLE VERTICAL BAR DOUBLE RIGHT TURNSTILE
    [0x22B2, 0x22EA], // NORMAL SUBGROUP OF -> NOT NORMAL SUBGROUP OF
    [0x22B3, 0x22EB], // CONTAINS AS NORMAL SUBGROUP -> DOES NOT CONTAIN AS NORMAL SUBGROUP
    [0x22B4, 0x22EC], // NORMAL SUBGROUP OF OR EQUAL TO -> NOT NORMAL SUBGROUP OF OR EQUAL TO
    [0x22B5, 0x22ED], // CONTAINS AS NORMAL SUBGROUP OR EQUAL TO -> DOES NOT CONTAIN AS NORMAL SUBGROUP OR EQUAL
    [0x22E0, 0x22E0], // DOES NOT PRECEDE OR EQUAL
    [0x22E1, 0x22E1], // DOES NOT SUCCEED OR EQUAL
    [0x22E2, 0x22E2], // NOT SQUARE IMAGE OF OR EQUAL TO
    [0x22E3, 0x22E3], // NOT SQUARE ORIGINAL OF OR EQUAL TO
    [0x22EA, 0x22EA], // NOT NORMAL SUBGROUP OF
    [0x22EB, 0x22EB], // DOES NOT CONTAIN AS NORMAL SUBGROUP
    [0x22EC, 0x22EC], // NOT NORMAL SUBGROUP OF OR EQUAL TO
    [0x22ED, 0x22ED], // DOES NOT CONTAIN AS NORMAL SUBGROUP OR EQUAL
]);

#[cfg(test)]
mod tests {
    use super::*;
    use VariationSelector::*;

    #[test]
    fn solidus_table_sanity_check() {
        // Check that the table is sorted
        assert!(PRECOMPOSED_SOLIDUS_OVERLAY.0.is_sorted_by_key(|a| a[0]));
        // Check that the table has an even number of entries
        assert!(PRECOMPOSED_SOLIDUS_OVERLAY.0.len().is_multiple_of(2));
        // Check that exactly half the entries are an idempotent mapping
        assert!(
            PRECOMPOSED_SOLIDUS_OVERLAY
                .0
                .iter()
                .filter(|[from, to]| from == to)
                .count()
                * 2
                == PRECOMPOSED_SOLIDUS_OVERLAY.0.len()
        );
    }

    /// Test every operation on a representative set of [`SuperChar`] values.
    ///
    /// `SuperChar`'s behavior depends on the base `char` only through its UTF-8
    /// length, whether it has a precomposed solidus-overlay form, and (for
    /// `from_char_with_vs`) whether it *is* such a precomposed form. The
    /// variation-selector/overlay operations are base-char-independent bit
    /// twiddling. So rather than exhaustively iterating all ~1.1M code points
    /// (which takes ~20s in debug builds), we test a curated set that exercises
    /// each of those cases.
    #[test]
    fn test_super_char() {
        let interesting = [
            // UTF-8 length boundaries (1/2/3/4 bytes) and surrogate boundaries
            '\u{0}',
            '\u{7F}',
            '\u{80}',
            '\u{7FF}',
            '\u{800}',
            '\u{D7FF}',
            '\u{E000}',
            '\u{FFFF}',
            '\u{10000}',
            char::MAX,
            // some ordinary characters not in the precomposed table
            'a',
            'Z',
            '0',
            'α',
            '∑',
            '€',
            '😀',
        ]
        .into_iter()
        // every character participating in the precomposed solidus-overlay table,
        // covering both the "has a precomposed form" and "is a precomposed form" cases
        .chain(
            PRECOMPOSED_SOLIDUS_OVERLAY
                .0
                .iter()
                .flat_map(|&[from, to]| [from, to])
                .map(|cp| char::from_u32(cp.into()).unwrap()),
        );

        for base in interesting {
            // Test base character alone

            let sc = SuperChar::from_char(base);
            assert!(sc.chars().eq([base]));
            assert_eq!(sc.base_char(), base);
            assert_eq!(sc.try_as_char(), Some(base));
            assert!(!sc.has_vs());
            assert_eq!(sc.vs(), None);

            let mut sc_buf = [255u8; 4];
            sc.encode_utf8(&mut sc_buf);
            let mut char_buf = [255u8; 4];
            base.encode_utf8(&mut char_buf);

            assert_eq!(sc_buf, char_buf);
            assert_eq!(sc.to_string(), base.to_string());

            // Test with solidus overlay

            let sc_solidus = sc.with_overlay(OverlayChar::Solidus);
            if let Some(precomposed) = get_precomposed_solidus_overlay(base) {
                assert_eq!(sc_solidus == sc, precomposed == base);
                assert!(sc_solidus.chars().eq([precomposed]));
                assert_eq!(sc_solidus.base_char(), precomposed);
                assert_eq!(sc_solidus.try_as_char(), Some(precomposed));
            } else {
                assert!(sc_solidus != sc);
                assert!(
                    sc_solidus
                        .chars()
                        .eq([base, symbol::COMBINING_LONG_SOLIDUS_OVERLAY])
                );
                assert_eq!(sc_solidus.base_char(), base);
                assert_eq!(sc_solidus.try_as_char(), None);

                let mut sc_buf_solidus = [255u8; 7];
                sc_solidus.encode_utf8(&mut sc_buf_solidus);
                let mut char_buf_solidus = [255u8; 7];
                let base_utf8_len = base.encode_utf8(&mut char_buf_solidus).len();
                symbol::COMBINING_LONG_SOLIDUS_OVERLAY
                    .encode_utf8(&mut char_buf_solidus[base_utf8_len..]);
                assert_eq!(sc_buf_solidus, char_buf_solidus);
            }
            assert!(!sc_solidus.has_vs());
            assert_eq!(sc_solidus.vs(), None);
            assert_eq!(sc_solidus.with_overlay(OverlayChar::Solidus), sc_solidus);

            // Test with vertical line overlay

            let sc_vert = sc.with_overlay(OverlayChar::VerticalLine);
            assert!(sc_vert != sc);
            assert!(sc_vert != sc_solidus);
            assert!(
                sc_vert
                    .chars()
                    .eq([base, symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY])
            );
            assert_eq!(sc_vert.base_char(), base);
            assert_eq!(sc_vert.try_as_char(), None);

            let mut sc_buf_vert = [255u8; 7];
            sc_vert.encode_utf8(&mut sc_buf_vert);
            let mut char_buf_vert = [255u8; 7];
            let base_utf8_len = base.encode_utf8(&mut char_buf_vert).len();
            symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                .encode_utf8(&mut char_buf_vert[base_utf8_len..]);
            assert_eq!(sc_buf_vert, char_buf_vert);

            assert!(!sc_vert.has_vs());
            assert_eq!(sc_vert.vs(), None);
            assert_eq!(sc_vert.with_overlay(OverlayChar::VerticalLine), sc_vert);

            // Test with both overlays

            let sc_both = sc_solidus.with_overlay(OverlayChar::VerticalLine);
            assert_eq!(sc_both, sc_vert.with_overlay(OverlayChar::Solidus));
            if let Some(precomposed) = get_precomposed_solidus_overlay(base) {
                assert_eq!(sc_both == sc_vert, precomposed == base);
                assert!(
                    sc_both
                        .chars()
                        .eq([precomposed, symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY])
                );
                assert_eq!(sc_both.base_char(), precomposed);
            } else {
                assert!(sc_both != sc_vert);
                assert!(sc_both.chars().eq([
                    base,
                    symbol::COMBINING_LONG_SOLIDUS_OVERLAY,
                    symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                ]));
                assert_eq!(sc_both.base_char(), base);

                let mut sc_buf_both = [255u8; 10];
                sc_both.encode_utf8(&mut sc_buf_both);
                let mut char_buf_both = [255u8; 10];
                let base_utf8_len = base.encode_utf8(&mut char_buf_both).len();
                symbol::COMBINING_LONG_SOLIDUS_OVERLAY
                    .encode_utf8(&mut char_buf_both[base_utf8_len..]);
                symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                    .encode_utf8(&mut char_buf_both[(base_utf8_len + 2)..]);
                assert_eq!(sc_buf_both, char_buf_both);
            }
            assert!(sc_both != sc_solidus);
            assert_eq!(sc_both.try_as_char(), None);
            assert!(!sc_both.has_vs());
            assert_eq!(sc_both.vs(), None);
            assert_eq!(sc_both.with_overlay(OverlayChar::Solidus), sc_both);
            assert_eq!(sc_both.with_overlay(OverlayChar::VerticalLine), sc_both);

            // Test with variation selector.
            // Characters with a precomposed solidus overlay shouldn't have a variation selector

            if !is_precomposed_solidus_overlay_for_debug(base) {
                for vs in [
                    Vs1, Vs2, Vs3, Vs4, Vs5, Vs6, Vs7, Vs8, Vs9, Vs10, Vs11, Vs12, Vs13, Vs14, Vs15,
                ] {
                    let sc_vs = SuperChar::from_char_with_vs(base, vs);

                    assert!(sc_vs != sc);
                    assert!(sc_vs.chars().eq([base, vs.into()]));
                    assert_eq!(sc_vs.base_char(), base);
                    assert_eq!(sc_vs.try_as_char(), None);
                    assert!(sc_vs.has_vs());
                    assert_eq!(sc_vs.vs(), Some(vs));

                    // Test with solidus overlay

                    let sc_vs_solidus = sc_vs.with_overlay(OverlayChar::Solidus);
                    assert!(sc_vs_solidus != sc_vs);
                    assert!(sc_vs_solidus.chars().eq([
                        base,
                        vs.into(),
                        symbol::COMBINING_LONG_SOLIDUS_OVERLAY
                    ]));
                    assert_eq!(sc_vs_solidus.base_char(), base);
                    assert_eq!(sc_vs_solidus.try_as_char(), None);

                    let mut sc_buf_vs_solidus = [255u8; 9];
                    sc_vs_solidus.encode_utf8(&mut sc_buf_vs_solidus);
                    let mut char_buf_vs_solidus = [255u8; 9];
                    let base_utf8_len = base.encode_utf8(&mut char_buf_vs_solidus).len();
                    char::from(vs).encode_utf8(&mut char_buf_vs_solidus[base_utf8_len..]);
                    symbol::COMBINING_LONG_SOLIDUS_OVERLAY
                        .encode_utf8(&mut char_buf_vs_solidus[(base_utf8_len + 3)..]);
                    assert_eq!(sc_buf_vs_solidus, char_buf_vs_solidus);

                    assert!(sc_vs_solidus.has_vs());
                    assert_eq!(sc_vs_solidus.vs(), Some(vs));
                    assert_eq!(
                        sc_vs_solidus.with_overlay(OverlayChar::Solidus),
                        sc_vs_solidus
                    );

                    // Test with vertical line overlay

                    let sc_vs_vert = sc_vs.with_overlay(OverlayChar::VerticalLine);
                    assert!(sc_vs_vert != sc_vs);
                    assert!(sc_vs_vert != sc_vs_solidus);
                    assert!(sc_vs_vert.chars().eq([
                        base,
                        vs.into(),
                        symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                    ]));
                    assert_eq!(sc_vs_vert.base_char(), base);
                    assert_eq!(sc_vs_vert.try_as_char(), None);

                    let mut sc_buf_vs_vert = [255u8; 10];
                    sc_vs_vert.encode_utf8(&mut sc_buf_vs_vert);
                    let mut char_buf_vs_vert = [255u8; 10];
                    let base_utf8_len = base.encode_utf8(&mut char_buf_vs_vert).len();
                    char::from(vs).encode_utf8(&mut char_buf_vs_vert[base_utf8_len..]);
                    symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                        .encode_utf8(&mut char_buf_vs_vert[(base_utf8_len + 3)..]);
                    assert_eq!(sc_buf_vs_vert, char_buf_vs_vert);

                    assert!(sc_vs_vert.has_vs());
                    assert_eq!(sc_vs_vert.vs(), Some(vs));
                    assert_eq!(
                        sc_vs_vert.with_overlay(OverlayChar::VerticalLine),
                        sc_vs_vert
                    );

                    // Test with both overlays

                    let sc_vs_both = sc_vs_solidus.with_overlay(OverlayChar::VerticalLine);
                    assert_eq!(sc_vs_both, sc_vs_vert.with_overlay(OverlayChar::Solidus));

                    assert!(sc_vs_both != sc_vs_solidus);
                    assert!(sc_vs_both != sc_vs_vert);
                    assert!(sc_vs_both.chars().eq([
                        base,
                        vs.into(),
                        symbol::COMBINING_LONG_SOLIDUS_OVERLAY,
                        symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                    ]));
                    assert_eq!(sc_vs_both.base_char(), base);

                    let mut sc_buf_vs_both = [255u8; 12];
                    sc_vs_both.encode_utf8(&mut sc_buf_vs_both);
                    let mut char_buf_vs_both = [255u8; 12];
                    let base_utf8_len = base.encode_utf8(&mut char_buf_vs_both).len();
                    char::from(vs).encode_utf8(&mut char_buf_vs_both[base_utf8_len..]);
                    symbol::COMBINING_LONG_SOLIDUS_OVERLAY
                        .encode_utf8(&mut char_buf_vs_both[(base_utf8_len + 3)..]);
                    symbol::COMBINING_LONG_VERTICAL_LINE_OVERLAY
                        .encode_utf8(&mut char_buf_vs_both[(base_utf8_len + 5)..]);
                    assert_eq!(sc_buf_vs_both, char_buf_vs_both);

                    assert_eq!(sc_vs_both.try_as_char(), None);
                    assert!(sc_vs_both.has_vs());
                    assert_eq!(sc_vs_both.vs(), Some(vs));
                    assert_eq!(sc_vs_both.with_overlay(OverlayChar::Solidus), sc_vs_both);
                    assert_eq!(
                        sc_vs_both.with_overlay(OverlayChar::VerticalLine),
                        sc_vs_both
                    );
                }
            }
        }
    }
}
