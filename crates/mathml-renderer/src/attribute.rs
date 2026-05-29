use bitflags::bitflags;
#[cfg(feature = "serde")]
use serde::Serialize;

use strum_macros::IntoStaticStr;

use crate::super_char::{SuperChar, VariationSelector};

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize))]
    pub struct OpAttrs: u8 {
        const STRETCHY_FALSE = 1;
        const STRETCHY_TRUE = 1 << 1;
        const NO_MOVABLE_LIMITS = 1 << 2;
        const FORCE_MOVABLE_LIMITS = 1 << 3;
        const FORM_PREFIX = 1 << 4;
        const FORM_INFIX = 1 << 5;
        const FORM_POSTFIX = 1 << 6;
        const SYMMETRIC_TRUE = 1 << 7;
    }
}

impl OpAttrs {
    pub fn write_to(self, s: &mut String) {
        debug_assert!(
            !(self.contains(OpAttrs::STRETCHY_FALSE) && self.contains(OpAttrs::STRETCHY_TRUE)),
            "STRETCHY_FALSE and STRETCHY_TRUE cannot both be set"
        );
        debug_assert!(
            !(self.contains(OpAttrs::NO_MOVABLE_LIMITS)
                && self.contains(OpAttrs::FORCE_MOVABLE_LIMITS)),
            "NO_MOVABLE_LIMITS and FORCE_MOVABLE_LIMITS cannot both be set"
        );
        debug_assert!(
            !(self.contains(OpAttrs::FORM_PREFIX) && self.contains(OpAttrs::FORM_POSTFIX)),
            "FORM_PREFIX and FORM_POSTFIX cannot both be set"
        );
        if self.contains(OpAttrs::STRETCHY_FALSE) {
            s.push_str(r#" stretchy="false""#);
        }
        if self.contains(OpAttrs::STRETCHY_TRUE) {
            s.push_str(r#" stretchy="true""#);
        }
        if self.contains(OpAttrs::NO_MOVABLE_LIMITS) {
            s.push_str(r#" movablelimits="false""#);
        }
        if self.contains(OpAttrs::FORCE_MOVABLE_LIMITS) {
            s.push_str(r#" movablelimits="true""#);
        }
        if self.contains(OpAttrs::FORM_PREFIX) {
            s.push_str(r#" form="prefix""#);
        }
        if self.contains(OpAttrs::FORM_INFIX) {
            s.push_str(r#" form="infix""#);
        }
        if self.contains(OpAttrs::FORM_POSTFIX) {
            s.push_str(r#" form="postfix""#);
        }
        if self.contains(OpAttrs::SYMMETRIC_TRUE) {
            s.push_str(r#" symmetric="true""#);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LetterAttr {
    Default,
    ForcedUpright,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Size {
    #[strum(serialize = "1.2em")]
    Scale1 = 1,
    #[strum(serialize = "1.623em")]
    Scale2,
    #[strum(serialize = "2.047em")]
    Scale3,
    #[strum(serialize = "2.470em")]
    Scale4,
}

/// display style
#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum FracAttr {
    #[strum(serialize = r#" displaystyle="true""#)]
    DisplayStyleTrue = 1,
    #[strum(serialize = r#" displaystyle="false""#)]
    DisplayStyleFalse,
    #[strum(serialize = r#" displaystyle="true" scriptlevel="0" style="padding-top: 0.1667em""#)]
    CFracStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Style {
    #[strum(serialize = r#" displaystyle="true" scriptlevel="0""#)]
    Display = 1,
    #[strum(serialize = r#" displaystyle="false" scriptlevel="0""#)]
    Text,
    #[strum(serialize = r#" displaystyle="false" scriptlevel="1""#)]
    Script,
    #[strum(serialize = r#" displaystyle="false" scriptlevel="2""#)]
    ScriptScript,
}

impl Style {
    /// One step smaller, as used for the numerator and denominator of a fraction.
    pub const fn shrink(self) -> Self {
        match self {
            Style::Display => Style::Text,
            Style::Text => Style::Script,
            Style::Script | Style::ScriptScript => Style::ScriptScript,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum MathSpacing {
    #[strum(serialize = "0")]
    Zero = 1,
    #[strum(serialize = "0.1667em")]
    ThreeMu, // 3/18 of an em/\quad
    #[strum(serialize = "0.2222em")]
    FourMu, // 4/18 of an em/\quad
    #[strum(serialize = "0.2778em")]
    FiveMu, // 5/18 of an em/\quad
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum RowAttr {
    Style(Style),
    Color(u8, u8, u8),
}

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize))]
    pub struct Notation: u8 {
        const HORIZONTAL = 1; // (not used at the moment)
        const UP_DIAGONAL = 1 << 1;
        const DOWN_DIAGONAL = 1 << 2;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum HtmlTextStyle {
    Bold = 1,
    Italic,
    BoldItalic,
    Emphasis,
    Typewriter,
    SmallCaps,
    SansSerif,
    Serif,
    Strikethrough,
}

// Transform of unicode characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum TextTransform {
    Bold = 1,
    BoldFraktur,
    BoldItalic,
    BoldSansSerif,
    BoldScript,
    DoubleStruck,
    Fraktur,
    // Initial,
    Italic,
    // Looped,
    Monospace,
    SansSerif,
    SansSerifBoldItalic,
    SansSerifItalic,
    ScriptChancery,
    ScriptRoundhand,
    // Stretched,
    // Tailed,
}

#[inline]
const fn add_offset(c: char, offset: u32) -> char {
    debug_assert!(char::from_u32(c as u32 + offset).is_some());
    // SAFETY: the offsets are such that the resulting char should be valid.
    unsafe { char::from_u32_unchecked(c as u32 + offset) }
}

impl TextTransform {
    /// If the string does not have length 1, it is passed through unchanged.
    // FIXME maybe we should do better than that?
    #[inline]
    pub const fn transform(self, ts: SuperChar, is_upright: bool) -> SuperChar {
        match ts.try_as_char() {
            Some(c) => self.transform_char(c, is_upright),
            None => ts,
        }
    }

    #[allow(clippy::manual_is_ascii_check)]
    pub const fn transform_char(self, c: char, is_upright: bool) -> SuperChar {
        let tf = if is_upright && matches!(self, TextTransform::BoldItalic) {
            TextTransform::Bold
        } else {
            self
        };
        let mapped = match tf {
            TextTransform::BoldScript => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D48F)),
                'a'..='z' => Some(add_offset(c, 0x1D489)),
                _ => None,
            },
            TextTransform::BoldItalic => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D427)),
                'a'..='z' => Some(add_offset(c, 0x1D421)),
                'Α'..='Ω' => Some(add_offset(c, 0x1D38B)),
                'α'..='ω' => Some(add_offset(c, 0x1D385)),
                'ϴ' => Some('𝜭'),
                '∇' => Some('𝜵'),
                '∂' => Some('𝝏'),
                'ϵ' => Some('𝝐'),
                'ϑ' => Some('𝝑'),
                'ϰ' => Some('𝝒'),
                'ϕ' => Some('𝝓'),
                'ϱ' => Some('𝝔'),
                'ϖ' => Some('𝝕'),
                _ => None,
            },
            TextTransform::Bold => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D3BF)),
                'a'..='z' => Some(add_offset(c, 0x1D3B9)),
                'Α'..='Ω' => Some(add_offset(c, 0x1D317)),
                'α'..='ω' => Some(add_offset(c, 0x1D311)),
                'Ϝ'..='ϝ' => Some(add_offset(c, 0x1D3EE)),
                '0'..='9' => Some(add_offset(c, 0x1D79E)),
                'ϴ' => Some('𝚹'),
                '∇' => Some('𝛁'),
                '∂' => Some('𝛛'),
                'ϵ' => Some('𝛜'),
                'ϑ' => Some('𝛝'),
                'ϰ' => Some('𝛞'),
                'ϕ' => Some('𝛟'),
                'ϱ' => Some('𝛠'),
                'ϖ' => Some('𝛡'),
                _ => None,
            },
            TextTransform::Fraktur => match c {
                'A'..='B' | 'D'..='G' | 'J'..='Q' | 'S'..='Y' => Some(add_offset(c, 0x1D4C3)),
                'H'..='I' => Some(add_offset(c, 0x20C4)),
                'a'..='z' => Some(add_offset(c, 0x1D4BD)),
                'C' => Some('ℭ'),
                'R' => Some('ℜ'),
                'Z' => Some('ℨ'),
                _ => None,
            },
            TextTransform::ScriptChancery | TextTransform::ScriptRoundhand => match c {
                'A' | 'C'..='D' | 'G' | 'J'..='K' | 'N'..='Q' | 'S'..='Z' => {
                    Some(add_offset(c, 0x1D45B))
                }
                'E'..='F' => Some(add_offset(c, 0x20EB)),
                'a'..='d' | 'f' | 'h'..='n' | 'p'..='z' => Some(add_offset(c, 0x1D455)),
                'B' => Some('ℬ'),
                'H' => Some('ℋ'),
                'I' => Some('ℐ'),
                'L' => Some('ℒ'),
                'M' => Some('ℳ'),
                'R' => Some('ℛ'),
                'e' => Some('ℯ'),
                'g' => Some('ℊ'),
                'o' => Some('ℴ'),
                _ => None,
            },
            TextTransform::Monospace => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D62F)),
                'a'..='z' => Some(add_offset(c, 0x1D629)),
                '0'..='9' => Some(add_offset(c, 0x1D7C6)),
                _ => None,
            },
            TextTransform::SansSerif => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D55F)),
                'a'..='z' => Some(add_offset(c, 0x1D559)),
                '0'..='9' => Some(add_offset(c, 0x1D7B2)),
                _ => None,
            },
            TextTransform::BoldFraktur => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D52B)),
                'a'..='z' => Some(add_offset(c, 0x1D525)),
                _ => None,
            },
            TextTransform::SansSerifBoldItalic => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D5FB)),
                'a'..='z' => Some(add_offset(c, 0x1D5F5)),
                'Α'..='Ω' => Some(add_offset(c, 0x1D3FF)),
                'α'..='ω' => Some(add_offset(c, 0x1D3F9)),
                'ϴ' => Some('𝞡'),
                '∇' => Some('𝞩'),
                '∂' => Some('𝟃'),
                'ϵ' => Some('𝟄'),
                'ϑ' => Some('𝟅'),
                'ϰ' => Some('𝟆'),
                'ϕ' => Some('𝟇'),
                'ϱ' => Some('𝟈'),
                'ϖ' => Some('𝟉'),
                _ => None,
            },
            TextTransform::SansSerifItalic => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D5C7)),
                'a'..='z' => Some(add_offset(c, 0x1D5C1)),
                _ => None,
            },
            TextTransform::BoldSansSerif => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D593)),
                'a'..='z' => Some(add_offset(c, 0x1D58D)),
                'Α'..='Ω' => Some(add_offset(c, 0x1D3C5)),
                'α'..='ω' => Some(add_offset(c, 0x1D3BF)),
                '0'..='9' => Some(add_offset(c, 0x1D7BC)),
                'ϴ' => Some('𝝧'),
                '∇' => Some('𝝯'),
                '∂' => Some('𝞉'),
                'ϵ' => Some('𝞊'),
                'ϑ' => Some('𝞋'),
                'ϰ' => Some('𝞌'),
                'ϕ' => Some('𝞍'),
                'ϱ' => Some('𝞎'),
                'ϖ' => Some('𝞏'),
                _ => None,
            },
            TextTransform::DoubleStruck => match c {
                'A'..='B' | 'D'..='G' | 'I'..='M' | 'O' | 'S'..='Y' => Some(add_offset(c, 0x1D4F7)),
                'P'..='Q' => Some(add_offset(c, 0x20C9)),
                'a'..='z' => Some(add_offset(c, 0x1D4F1)),
                '0'..='9' => Some(add_offset(c, 0x1D7A8)),
                'C' => Some('ℂ'),
                'H' => Some('ℍ'),
                'N' => Some('ℕ'),
                'R' => Some('ℝ'),
                'Z' => Some('ℤ'),
                'π' => Some('ℼ'),
                'γ' => Some('ℽ'),
                'Γ' => Some('ℾ'),
                'Π' => Some('ℿ'),
                '∑' => Some('⅀'),
                // FIXME: add Arabic double-struck characters
                _ => None,
            },
            TextTransform::Italic => match c {
                'A'..='Z' => Some(add_offset(c, 0x1D3F3)),
                'a'..='g' | 'i'..='z' => Some(add_offset(c, 0x1D3ED)),
                'Α'..='Ω' => Some(add_offset(c, 0x1D351)),
                'α'..='ω' => Some(add_offset(c, 0x1D34B)),
                'h' => Some('ℎ'),
                'ı' => Some('𝚤'),
                'ȷ' => Some('𝚥'),
                'ϴ' => Some('𝛳'),
                '∇' => Some('𝛻'),
                '∂' => Some('𝜕'),
                'ϵ' => Some('𝜖'),
                'ϑ' => Some('𝜗'),
                'ϰ' => Some('𝜘'),
                'ϕ' => Some('𝜙'),
                'ϱ' => Some('𝜚'),
                'ϖ' => Some('𝜛'),
                _ => None,
            },
        };

        match mapped {
            Some(mapped_char) => match tf {
                TextTransform::ScriptChancery => {
                    SuperChar::from_char_with_vs(mapped_char, VariationSelector::Vs1)
                }
                TextTransform::ScriptRoundhand => {
                    SuperChar::from_char_with_vs(mapped_char, VariationSelector::Vs2)
                }
                _ => SuperChar::from_char(mapped_char),
            },
            None => SuperChar::from_char(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::super_char::{SuperChar, VariationSelector};

    use super::TextTransform;

    #[test]
    fn transform_test() {
        let problems: [(char, TextTransform, SuperChar); _] = [
            ('G', TextTransform::BoldScript, '𝓖'.into()),
            ('H', TextTransform::Italic, '𝐻'.into()),
            ('X', TextTransform::Fraktur, '𝔛'.into()),
            (
                'S',
                TextTransform::ScriptChancery,
                SuperChar::from_char_with_vs('𝒮', VariationSelector::Vs1),
            ),
            (
                'M',
                TextTransform::ScriptRoundhand,
                SuperChar::from_char_with_vs('ℳ', VariationSelector::Vs2),
            ),
            ('f', TextTransform::Bold, '𝐟'.into()),
            ('g', TextTransform::Bold, '𝐠'.into()),
            ('o', TextTransform::DoubleStruck, '𝕠'.into()),
            ('D', TextTransform::Monospace, '𝙳'.into()),
            ('x', TextTransform::Monospace, '𝚡'.into()),
            ('2', TextTransform::Monospace, '𝟸'.into()),
            ('U', TextTransform::SansSerif, '𝖴'.into()),
            ('v', TextTransform::SansSerif, '𝗏'.into()),
            ('4', TextTransform::SansSerif, '𝟦'.into()),
            ('A', TextTransform::SansSerifBoldItalic, '𝘼'.into()),
            ('a', TextTransform::SansSerifBoldItalic, '𝙖'.into()),
            ('Α', TextTransform::SansSerifBoldItalic, '𝞐'.into()),
            ('α', TextTransform::SansSerifBoldItalic, '𝞪'.into()),
            ('A', TextTransform::SansSerifItalic, '𝘈'.into()),
            ('a', TextTransform::SansSerifItalic, '𝘢'.into()),
            ('J', TextTransform::BoldSansSerif, '𝗝'.into()),
            ('r', TextTransform::BoldSansSerif, '𝗿'.into()),
            ('Ξ', TextTransform::BoldSansSerif, '𝝣'.into()),
            ('τ', TextTransform::BoldSansSerif, '𝞃'.into()),
        ];
        for (source, transform, target) in problems.into_iter() {
            assert_eq!(
                target,
                transform.transform_char(source, false),
                "executed: {:?}({})",
                transform,
                source
            );
        }
    }
}
