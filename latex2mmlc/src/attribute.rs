use strum_macros::AsRefStr;

/// mi mathvariant attribute
#[derive(Debug, Clone, PartialEq)]
pub enum MathVariant {
    Normal = 1,
}

impl AsRef<str> for MathVariant {
    fn as_ref(&self) -> &str {
        match self {
            MathVariant::Normal => r#" mathvariant="normal""#,
        }
    }
}

#[derive(Debug, PartialEq, AsRefStr)]
pub enum Accent {
    #[strum(serialize = "true")]
    True,
    #[strum(serialize = "false")]
    False,
}

#[derive(Debug, PartialEq, AsRefStr)]
pub enum LineThickness {
    #[strum(serialize = "")]
    Medium,
    #[strum(serialize = r#" linethickness="0""#)]
    Zero,
}

#[derive(Debug, PartialEq, AsRefStr)]
pub enum OpAttr {
    #[strum(serialize = r#" stretchy="true""#)]
    StretchyTrue = 1,
    #[strum(serialize = r#" stretchy="false""#)]
    StretchyFalse,
    #[strum(serialize = r#" movablelimits="false""#)]
    NoMovableLimits,
}

/// display style
#[derive(Debug, PartialEq, AsRefStr)]
pub enum DisplayStyle {
    #[strum(serialize = r#" displaystyle="true""#)]
    True = 1,
    #[strum(serialize = r#" displaystyle="false""#)]
    False = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Align {
    Center,
    Left,
    Alternating,
}

#[derive(Debug, Clone, PartialEq, AsRefStr)]
pub enum PhantomWidth {
    #[strum(serialize = " style=\"width:0\"")]
    Zero,
    #[strum(serialize = "")]
    Default,
}

/// <mi> mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextTransform {
    Bold,
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
    Script,
    // Stretched,
    // Tailed,
}

macro_rules! transform {
    ($char:expr, [ $( ($range:expr, $offset:expr) ),* $(,)? ], [ $( ($index:expr, $target:expr) ),* $(,)? ] ) => {
        'transform_block: {
            $(
                if $range.contains(&$char) {
                    break 'transform_block std::char::from_u32($char as u32 + $offset).unwrap();
                }
            )*
            $(
                if $char == $index {
                    break 'transform_block $target;
                }
            )*
            $char
        }
    };
}

impl TextTransform {
    #[allow(clippy::manual_is_ascii_check)]
    pub fn transform(&self, c: char) -> char {
        match self {
            TextTransform::BoldScript => {
                transform!(c, [('A'..='Z', 0x1D48F), ('a'..='z', 0x1D489)], [])
            }
            TextTransform::BoldItalic => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D427),
                        ('a'..='z', 0x1D421),
                        ('Α'..='Ρ', 0x1D38B),
                        ('Σ'..='Ω', 0x1D38B),
                        ('α'..='ω', 0x1D385),
                    ],
                    [
                        ('ϴ', '𝜭'),
                        ('∇', '𝜵'),
                        ('∂', '𝝏'),
                        ('ϵ', '𝝐'),
                        ('ϑ', '𝝑'),
                        ('ϰ', '𝝒'),
                        ('ϕ', '𝝓'),
                        ('ϱ', '𝝔'),
                        ('ϖ', '𝝕'),
                    ]
                )
            }
            TextTransform::Bold => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D3BF),
                        ('a'..='z', 0x1D3B9),
                        ('Α'..='Ρ', 0x1D317),
                        ('Σ'..='Ω', 0x1D317),
                        ('α'..='ω', 0x1D311),
                        ('Ϝ'..='ϝ', 0x1D3EE),
                        ('0'..='9', 0x1D79E),
                    ],
                    [
                        ('ϴ', '𝚹'),
                        ('∇', '𝛁'),
                        ('∂', '𝛛'),
                        ('ϵ', '𝛜'),
                        ('ϑ', '𝛝'),
                        ('ϰ', '𝛞'),
                        ('ϕ', '𝛟'),
                        ('ϱ', '𝛠'),
                        ('ϖ', '𝛡'),
                    ]
                )
            }
            TextTransform::Fraktur => {
                transform!(
                    c,
                    [
                        ('A'..='B', 0x1D4C3),
                        ('D'..='G', 0x1D4C3),
                        ('H'..='I', 0x20C4),
                        ('J'..='Q', 0x1D4C3),
                        ('S'..='Y', 0x1D4C3),
                        ('a'..='z', 0x1D4BD),
                    ],
                    [('C', 'ℭ'), ('R', 'ℜ'), ('Z', 'ℨ')]
                )
            }
            TextTransform::Script => {
                transform!(
                    c,
                    [
                        ('C'..='D', 0x1D45B),
                        ('E'..='F', 0x20EB),
                        ('H'..='I', 0x20C3),
                        ('J'..='K', 0x1D45B),
                        ('N'..='Q', 0x1D45B),
                        ('S'..='Z', 0x1D45B),
                        ('a'..='d', 0x1D455),
                        ('h'..='n', 0x1D455),
                        ('p'..='z', 0x1D455),
                    ],
                    [
                        ('A', '𝒜'),
                        ('B', 'ℬ'),
                        ('G', '𝒢'),
                        ('L', 'ℒ'),
                        ('M', 'ℳ'),
                        ('R', 'ℛ'),
                        ('e', 'ℯ'),
                        ('f', '𝒻'),
                        ('g', 'ℊ'),
                        ('o', 'ℴ'),
                    ]
                )
            }
            TextTransform::Monospace => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D62F),
                        ('a'..='z', 0x1D629),
                        ('0'..='9', 0x1D7C6),
                    ],
                    []
                )
            }
            TextTransform::SansSerif => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D55F),
                        ('a'..='z', 0x1D559),
                        ('0'..='9', 0x1D7B2),
                    ],
                    []
                )
            }
            TextTransform::BoldFraktur => {
                transform!(c, [('A'..='Z', 0x1D52B), ('a'..='z', 0x1D525)], [])
            }
            TextTransform::SansSerifBoldItalic => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D5FB),
                        ('a'..='z', 0x1D5F5),
                        ('Α'..='Ρ', 0x1D3FF),
                        ('Σ'..='Ω', 0x1D3FF),
                        ('α'..='ω', 0x1D3F9),
                    ],
                    [
                        ('ϴ', '𝞡'),
                        ('∇', '𝞩'),
                        ('∂', '𝟃'),
                        ('ϵ', '𝟄'),
                        ('ϑ', '𝟅'),
                        ('ϰ', '𝟆'),
                        ('ϕ', '𝟇'),
                        ('ϱ', '𝟈'),
                        ('ϖ', '𝟉'),
                    ]
                )
            }
            TextTransform::SansSerifItalic => {
                transform!(c, [('A'..='Z', 0x1D5C7), ('a'..='z', 0x1D5C1)], [])
            }
            TextTransform::BoldSansSerif => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D593),
                        ('a'..='z', 0x1D58D),
                        ('Α'..='Ρ', 0x1D3C5),
                        ('Σ'..='Ω', 0x1D3C5),
                        ('α'..='ω', 0x1D3BF),
                        ('0'..='9', 0x1D7BC),
                    ],
                    [
                        ('ϴ', '𝝧'),
                        ('∇', '𝝯'),
                        ('∂', '𝞉'),
                        ('ϵ', '𝞊'),
                        ('ϑ', '𝞋'),
                        ('ϰ', '𝞌'),
                        ('ϕ', '𝞍'),
                        ('ϱ', '𝞎'),
                        ('ϖ', '𝞏'),
                    ]
                )
            }
            TextTransform::DoubleStruck => {
                transform!(
                    c,
                    [
                        ('A'..='B', 0x1D4F7),
                        ('D'..='G', 0x1D4F7),
                        ('I'..='M', 0x1D4F7),
                        ('P'..='Q', 0x20C9),
                        ('S'..='Y', 0x1D4F7),
                        ('a'..='z', 0x1D4F1),
                        ('0'..='9', 0x1D7A8),
                    ],
                    [('C', 'ℂ'), ('H', 'ℍ'), ('N', 'ℕ'), ('R', 'ℝ'), ('Z', 'ℤ')]
                )
            }
            TextTransform::Italic => {
                transform!(
                    c,
                    [
                        ('A'..='Z', 0x1D3F3),
                        ('a'..='g', 0x1D3ED),
                        ('i'..='z', 0x1D3ED),
                        ('Α'..='Ρ', 0x1D351),
                        ('Σ'..='Ω', 0x1D351),
                        ('α'..='ω', 0x1D34B),
                    ],
                    [
                        ('h', 'ℎ'),
                        ('ı', '𝚤'),
                        ('ȷ', '𝚥'),
                        ('ϴ', '𝛳'),
                        ('∇', '𝛻'),
                        ('∂', '𝜕'),
                        ('ϵ', '𝜖'),
                        ('ϑ', '𝜗'),
                        ('ϰ', '𝜘'),
                        ('ϕ', '𝜙'),
                        ('ϱ', '𝜚'),
                        ('ϖ', '𝜛'),
                    ]
                )
            }
        }
    }
}
