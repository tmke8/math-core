use bitflags::bitflags;
#[cfg(feature = "serde")]
use serde::Serialize;

use strum_macros::IntoStaticStr;

/// <mi> mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum MathVariant {
    /// This is enforced by setting `mathvariant="normal"`.
    Normal,
    /// This is enforced by transforming the characters themselves.
    Transform(TextTransform),
}

impl MathVariant {
    /// Returns `true` if the transformation is sensitive to whether a letter is "upright".
    /// An example of an upright letter is "\Alpha".
    #[inline]
    pub fn differs_on_upright_letters(&self) -> bool {
        matches!(self, MathVariant::Transform(TextTransform::BoldItalic))
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize))]
    pub struct OpAttrs: u8 {
        const STRETCHY_FALSE = 1;
        const STRETCHY_TRUE = 1 << 1;
        const NO_MOVABLE_LIMITS = 1 << 2;
        const FORCE_MOVABLE_LIMITS = 1 << 3;
        const FORM_PREFIX = 1 << 4;
        const FORM_POSTFIX = 1 << 5;
    }
}

impl OpAttrs {
    pub fn write_to(self, s: &mut String) {
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
        if self.contains(OpAttrs::FORM_POSTFIX) {
            s.push_str(r#" form="postfix""#);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum LetterAttr {
    Default,
    ForcedUpright,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Size {
    #[strum(serialize = "1.2em")]
    Scale1,
    #[strum(serialize = "1.623em")]
    Scale2,
    #[strum(serialize = "2.047em")]
    Scale3,
    #[strum(serialize = "2.470em")]
    Scale4,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ParenType {
    #[strum(serialize = r#" form="prefix""#)]
    Open = 1,
    #[strum(serialize = r#" form="postfix""#)]
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum StretchMode {
    /// Don't stretch the operator.
    NoStretch = 1,
    /// Operator is in a fence and should stretch.
    Fence,
    /// Operator is in the middle of a fenced expression and should stretch.
    Middle,
}

/// display style
#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum FracAttr {
    #[strum(serialize = r#" displaystyle="true""#)]
    DisplayStyleTrue = 1,
    #[strum(serialize = r#" displaystyle="false""#)]
    DisplayStyleFalse,
    #[strum(serialize = r#" displaystyle="true" scriptlevel="0" style="padding-top: 0.1667em""#)]
    CFracStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
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

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
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

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum RowAttr {
    Style(Style),
    Color(u8, u8, u8),
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize))]
    pub struct Notation: u8 {
        const HORIZONTAL = 1;
        const UP_DIAGONAL = 1 << 1;
        const DOWN_DIAGONAL = 1 << 2;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
}

// Transform of unicode characters.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    #[allow(clippy::manual_is_ascii_check)]
    pub const fn transform(&self, c: char, is_upright: bool) -> char {
        let tf = if is_upright && matches!(self, TextTransform::BoldItalic) {
            &TextTransform::Bold
        } else {
            self
        };
        match tf {
            TextTransform::BoldScript => match c {
                'A'..='Z' => add_offset(c, 0x1D48F),
                'a'..='z' => add_offset(c, 0x1D489),
                _ => c,
            },
            TextTransform::BoldItalic => match c {
                'A'..='Z' => add_offset(c, 0x1D427),
                'a'..='z' => add_offset(c, 0x1D421),
                'Α'..='Ω' => add_offset(c, 0x1D38B),
                'α'..='ω' => add_offset(c, 0x1D385),
                'ϴ' => '𝜭',
                '∇' => '𝜵',
                '∂' => '𝝏',
                'ϵ' => '𝝐',
                'ϑ' => '𝝑',
                'ϰ' => '𝝒',
                'ϕ' => '𝝓',
                'ϱ' => '𝝔',
                'ϖ' => '𝝕',
                _ => c,
            },
            TextTransform::Bold => match c {
                'A'..='Z' => add_offset(c, 0x1D3BF),
                'a'..='z' => add_offset(c, 0x1D3B9),
                'Α'..='Ω' => add_offset(c, 0x1D317),
                'α'..='ω' => add_offset(c, 0x1D311),
                'Ϝ'..='ϝ' => add_offset(c, 0x1D3EE),
                '0'..='9' => add_offset(c, 0x1D79E),
                'ϴ' => '𝚹',
                '∇' => '𝛁',
                '∂' => '𝛛',
                'ϵ' => '𝛜',
                'ϑ' => '𝛝',
                'ϰ' => '𝛞',
                'ϕ' => '𝛟',
                'ϱ' => '𝛠',
                'ϖ' => '𝛡',
                _ => c,
            },
            TextTransform::Fraktur => match c {
                'A'..='B' | 'D'..='G' | 'J'..='Q' | 'S'..='Y' => add_offset(c, 0x1D4C3),
                'H'..='I' => add_offset(c, 0x20C4),
                'a'..='z' => add_offset(c, 0x1D4BD),
                'C' => 'ℭ',
                'R' => 'ℜ',
                'Z' => 'ℨ',
                _ => c,
            },
            TextTransform::ScriptChancery | TextTransform::ScriptRoundhand => match c {
                'A' | 'C'..='D' | 'G' | 'J'..='K' | 'N'..='Q' | 'S'..='Z' => add_offset(c, 0x1D45B),
                'E'..='F' => add_offset(c, 0x20EB),
                'a'..='d' | 'f' | 'h'..='n' | 'p'..='z' => add_offset(c, 0x1D455),
                'B' => 'ℬ',
                'H' => 'ℋ',
                'I' => 'ℐ',
                'L' => 'ℒ',
                'M' => 'ℳ',
                'R' => 'ℛ',
                'e' => 'ℯ',
                'g' => 'ℊ',
                'o' => 'ℴ',
                _ => c,
            },
            TextTransform::Monospace => match c {
                'A'..='Z' => add_offset(c, 0x1D62F),
                'a'..='z' => add_offset(c, 0x1D629),
                '0'..='9' => add_offset(c, 0x1D7C6),
                _ => c,
            },
            TextTransform::SansSerif => match c {
                'A'..='Z' => add_offset(c, 0x1D55F),
                'a'..='z' => add_offset(c, 0x1D559),
                '0'..='9' => add_offset(c, 0x1D7B2),
                _ => c,
            },
            TextTransform::BoldFraktur => match c {
                'A'..='Z' => add_offset(c, 0x1D52B),
                'a'..='z' => add_offset(c, 0x1D525),
                _ => c,
            },
            TextTransform::SansSerifBoldItalic => match c {
                'A'..='Z' => add_offset(c, 0x1D5FB),
                'a'..='z' => add_offset(c, 0x1D5F5),
                'Α'..='Ω' => add_offset(c, 0x1D3FF),
                'α'..='ω' => add_offset(c, 0x1D3F9),
                'ϴ' => '𝞡',
                '∇' => '𝞩',
                '∂' => '𝟃',
                'ϵ' => '𝟄',
                'ϑ' => '𝟅',
                'ϰ' => '𝟆',
                'ϕ' => '𝟇',
                'ϱ' => '𝟈',
                'ϖ' => '𝟉',
                _ => c,
            },
            TextTransform::SansSerifItalic => match c {
                'A'..='Z' => add_offset(c, 0x1D5C7),
                'a'..='z' => add_offset(c, 0x1D5C1),
                _ => c,
            },
            TextTransform::BoldSansSerif => match c {
                'A'..='Z' => add_offset(c, 0x1D593),
                'a'..='z' => add_offset(c, 0x1D58D),
                'Α'..='Ω' => add_offset(c, 0x1D3C5),
                'α'..='ω' => add_offset(c, 0x1D3BF),
                '0'..='9' => add_offset(c, 0x1D7BC),
                'ϴ' => '𝝧',
                '∇' => '𝝯',
                '∂' => '𝞉',
                'ϵ' => '𝞊',
                'ϑ' => '𝞋',
                'ϰ' => '𝞌',
                'ϕ' => '𝞍',
                'ϱ' => '𝞎',
                'ϖ' => '𝞏',
                _ => c,
            },
            TextTransform::DoubleStruck => match c {
                'A'..='B' | 'D'..='G' | 'I'..='M' | 'O' | 'S'..='Y' => add_offset(c, 0x1D4F7),
                'P'..='Q' => add_offset(c, 0x20C9),
                'a'..='z' => add_offset(c, 0x1D4F1),
                '0'..='9' => add_offset(c, 0x1D7A8),
                'C' => 'ℂ',
                'H' => 'ℍ',
                'N' => 'ℕ',
                'R' => 'ℝ',
                'Z' => 'ℤ',
                _ => c,
            },
            TextTransform::Italic => match c {
                'A'..='Z' => add_offset(c, 0x1D3F3),
                'a'..='g' | 'i'..='z' => add_offset(c, 0x1D3ED),
                'Α'..='Ω' => add_offset(c, 0x1D351),
                'α'..='ω' => add_offset(c, 0x1D34B),
                'h' => 'ℎ',
                'ı' => '𝚤',
                'ȷ' => '𝚥',
                'ϴ' => '𝛳',
                '∇' => '𝛻',
                '∂' => '𝜕',
                'ϵ' => '𝜖',
                'ϑ' => '𝜗',
                'ϰ' => '𝜘',
                'ϕ' => '𝜙',
                'ϱ' => '𝜚',
                'ϖ' => '𝜛',
                _ => c,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MathVariant, TextTransform};

    #[test]
    fn transform_test() {
        let problems = [
            ('G', TextTransform::BoldScript, '𝓖'),
            ('H', TextTransform::Italic, '𝐻'),
            ('X', TextTransform::Fraktur, '𝔛'),
            ('S', TextTransform::ScriptChancery, '𝒮'),
            ('f', TextTransform::Bold, '𝐟'),
            ('g', TextTransform::Bold, '𝐠'),
            ('o', TextTransform::DoubleStruck, '𝕠'),
            ('D', TextTransform::Monospace, '𝙳'),
            ('x', TextTransform::Monospace, '𝚡'),
            ('2', TextTransform::Monospace, '𝟸'),
            ('U', TextTransform::SansSerif, '𝖴'),
            ('v', TextTransform::SansSerif, '𝗏'),
            ('4', TextTransform::SansSerif, '𝟦'),
            ('A', TextTransform::SansSerifBoldItalic, '𝘼'),
            ('a', TextTransform::SansSerifBoldItalic, '𝙖'),
            ('Α', TextTransform::SansSerifBoldItalic, '𝞐'),
            ('α', TextTransform::SansSerifBoldItalic, '𝞪'),
            ('A', TextTransform::SansSerifItalic, '𝘈'),
            ('a', TextTransform::SansSerifItalic, '𝘢'),
            ('J', TextTransform::BoldSansSerif, '𝗝'),
            ('r', TextTransform::BoldSansSerif, '𝗿'),
            ('Ξ', TextTransform::BoldSansSerif, '𝝣'),
            ('τ', TextTransform::BoldSansSerif, '𝞃'),
        ];
        for (source, transform, target) in problems.into_iter() {
            assert_eq!(
                target,
                transform.transform(source, false),
                "executed: {:?}({})",
                transform,
                source
            );
        }
    }

    #[test]
    fn size_test() {
        assert_eq!(
            std::mem::size_of::<MathVariant>(),
            std::mem::size_of::<TextTransform>()
        );
        assert_eq!(
            std::mem::size_of::<Option<MathVariant>>(),
            std::mem::size_of::<TextTransform>()
        );
    }
}
