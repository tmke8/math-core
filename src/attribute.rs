use std::fmt;

/// mi mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MathVariant {
    Normal = 1,
}

impl fmt::Display for MathVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MathVariant::Normal => write!(f, r#" mathvariant="normal""#),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Accent {
    True,
    False,
}

impl fmt::Display for Accent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Accent::True => write!(f, "true"),
            Accent::False => write!(f, "false"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineThickness {
    Thin,
    Medium,
    Thick,
    Length(u8),
}
impl fmt::Display for LineThickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineThickness::Thin => write!(f, r#" linethickness="thin""#),
            LineThickness::Medium => write!(f, r#""#),
            LineThickness::Thick => write!(f, r#" linethickness="medium""#),
            LineThickness::Length(l) => write!(f, r#" linethickness="{}""#, l),
        }
    }
}

/// display style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayStyle {
    True = 1,
    False = 2,
}

impl fmt::Display for DisplayStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayStyle::True => write!(f, r#" displaystyle="true""#),
            DisplayStyle::False => write!(f, r#" displaystyle="false""#),
        }
    }
}

/// mi mathvariant attribute
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
    ($char:expr, [ $( ($start:expr, $end:expr, $offset:expr) ),* $(,)? ], [ $( ($index:expr, $target:expr) ),* $(,)? ] ) => {
        'transform_block: {
            $(
                if $char >= $start && $char <= $end {
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
    pub fn transform(&self, c: char) -> char {
        match self {
            TextTransform::BoldScript => {
                transform!(c, [('A', 'Z', 0x1D48F), ('a', 'z', 0x1D489)], [])
            }
            TextTransform::BoldItalic => {
                transform!(
                    c,
                    [
                        ('A', 'Z', 0x1D427),
                        ('a', 'z', 0x1D421),
                        ('Α', 'Ρ', 0x1D38B),
                        ('Σ', 'Ω', 0x1D38B),
                        ('α', 'ω', 0x1D385),
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
                        ('A', 'Z', 0x1D3BF),
                        ('a', 'z', 0x1D3B9),
                        ('Α', 'Ρ', 0x1D317),
                        ('Σ', 'Ω', 0x1D317),
                        ('α', 'ω', 0x1D311),
                        ('Ϝ', 'ϝ', 0x1D3EE),
                        ('0', '9', 0x1D79E),
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
                        ('A', 'B', 0x1D4C3),
                        ('D', 'G', 0x1D4C3),
                        ('H', 'I', 0x20C4),
                        ('J', 'Q', 0x1D4C3),
                        ('S', 'Y', 0x1D4C3),
                        ('a', 'z', 0x1D4BD),
                    ],
                    [('C', 'ℭ'), ('R', 'ℜ'), ('Z', 'ℨ')]
                )
            }
            TextTransform::Script => {
                transform!(
                    c,
                    [
                        ('C', 'D', 0x1D45B),
                        ('E', 'F', 0x20EB),
                        ('H', 'I', 0x20C3),
                        ('J', 'K', 0x1D45B),
                        ('N', 'Q', 0x1D45B),
                        ('S', 'Z', 0x1D45B),
                        ('a', 'd', 0x1D455),
                        ('h', 'n', 0x1D455),
                        ('p', 'z', 0x1D455),
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
                        ('A', 'Z', 0x1D62F),
                        ('a', 'z', 0x1D629),
                        ('0', '9', 0x1D7C6),
                    ],
                    []
                )
            }
            TextTransform::SansSerif => {
                transform!(
                    c,
                    [
                        ('A', 'Z', 0x1D55F),
                        ('a', 'z', 0x1D559),
                        ('0', '9', 0x1D7B2),
                    ],
                    []
                )
            }
            TextTransform::BoldFraktur => {
                transform!(c, [('A', 'Z', 0x1D52B), ('a', 'z', 0x1D525)], [])
            }
            TextTransform::SansSerifBoldItalic => {
                transform!(
                    c,
                    [
                        ('A', 'Z', 0x1D5FB),
                        ('a', 'z', 0x1D5F5),
                        ('Α', 'Ρ', 0x1D3FF),
                        ('Σ', 'Ω', 0x1D3FF),
                        ('α', 'ω', 0x1D3F9),
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
                transform!(c, [('A', 'Z', 0x1D5C7), ('a', 'z', 0x1D5C1)], [])
            }
            TextTransform::BoldSansSerif => {
                transform!(
                    c,
                    [
                        ('A', 'Z', 0x1D593),
                        ('a', 'z', 0x1D58D),
                        ('Α', 'Ρ', 0x1D3C5),
                        ('Σ', 'Ω', 0x1D3C5),
                        ('α', 'ω', 0x1D3BF),
                        ('0', '9', 0x1D7BC),
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
                        ('A', 'B', 0x1D4F7),
                        ('D', 'G', 0x1D4F7),
                        ('I', 'M', 0x1D4F7),
                        ('P', 'Q', 0x20C9),
                        ('S', 'Y', 0x1D4F7),
                        ('a', 'z', 0x1D4F1),
                        ('0', '9', 0x1D7A8),
                    ],
                    [('C', 'ℂ'), ('H', 'ℍ'), ('N', 'ℕ'), ('R', 'ℝ'), ('Z', 'ℤ')]
                )
            }
            TextTransform::Italic => {
                transform!(
                    c,
                    [
                        ('A', 'Z', 0x1D3F3),
                        ('a', 'g', 0x1D3ED),
                        ('i', 'z', 0x1D3ED),
                        ('Α', 'Ρ', 0x1D351),
                        ('Σ', 'Ω', 0x1D351),
                        ('α', 'ω', 0x1D34B),
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
