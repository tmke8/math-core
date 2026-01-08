use std::fmt::Debug;

#[cfg(feature = "serde")]
use serde::Serialize;

/// A character for use in MathML `<mo>` elements.
///
/// LaTeX's operator classes cannot be mapped cleanly to MathML's `<mo>` vs `<mi>` distinction.
/// Some characters that are in class 0 in LaTeX are nevertheless rendered as `<mo>` in MathML.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[repr(transparent)]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct MathMLOperator(char);

impl MathMLOperator {
    #[inline(always)]
    pub const fn as_char(&self) -> char {
        self.0
    }
}

impl From<MathMLOperator> for char {
    #[inline]
    fn from(op: MathMLOperator) -> Self {
        op.0
    }
}

impl From<&MathMLOperator> for char {
    #[inline]
    fn from(op: &MathMLOperator) -> Self {
        op.0
    }
}

/// A character from the Basic Multilingual Plane (BMP), stored as a single UTF-16 code unit.
///
/// This can only store characters in the BMP (i.e., code points U+0000 to U+FFFF).
/// `BMPChar::new` will panic if a character outside this range is provided.
#[derive(Clone, PartialEq, Eq, Copy)]
struct BMPChar {
    char: u16,
}

impl BMPChar {
    const fn new(ch: char) -> Self {
        assert!(ch as u32 <= u16::MAX as u32, "Character is outside the BMP");
        BMPChar { char: ch as u16 }
    }

    #[inline(always)]
    pub const fn as_char(&self) -> char {
        debug_assert!(char::from_u32(self.char as u32).is_some(), "Invalid char.");
        unsafe { char::from_u32_unchecked(self.char as u32) }
    }
}

// We use a custom `Debug` implementation to render `BMPChar` as a `char`.
impl Debug for BMPChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.as_char()))
    }
}

macro_rules! make_character_class {
    ($(#[$meta:meta])* $struct_name:ident, $cat_type:ty) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Copy)]
        pub struct $struct_name {
            char: BMPChar,
            cat: $cat_type,
        }

        impl $struct_name {
            const fn new(ch: char, cat: $cat_type) -> $struct_name {
                $struct_name {
                    char: BMPChar::new(ch),
                    cat,
                }
            }

            #[inline(always)]
            pub const fn as_op(&self) -> MathMLOperator {
                MathMLOperator(self.char.as_char())
            }

            #[inline]
            pub fn category(&self) -> &$cat_type {
                &self.cat
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdCategory {
    /// Category D: Prefix, zero spacing (e.g. `¬`).
    D,
    /// Category E: Postfix, zero spacing (e.g. `′`).
    E,
    /// Category F: Prefix or postfix, zero spacing, stretchy, symmetric
    F,
    /// Category G: Prefix or postfix, zero spacing, stretchy, symmetric
    G,
    /// Category F and G: Prefix or postfix, zero spacing, stretchy, symmetric
    /// (e.g. `‖`).
    // FG,

    /// Category F and G: Prefix or postfix, zero spacing, stretchy, symmetric
    /// but also with an infix form with relation spacing (e.g. `|`).
    FGandForceDefault,
    /// Category I: Postfix, zero spacing, stretchy
    I,
    /// Category K: Infix, zero spacing (e.g. `\`).
    K,
    /// An operator which is in category K but used to be in category B (e.g. `/`).
    /// Unfortunately, not all browsers handle this correctly yet,
    /// so we need to special-case it.
    KButUsedToBeB,
}

make_character_class!(
    /// An operator with zero spacing, categories D, E, F, G, I, K.
    OrdLike, OrdCategory
);

impl OrdLike {
    #[inline(always)]
    pub const fn as_stretchable_op(&self) -> Option<StretchableOp> {
        let (stretchy, nonzero_spacing) = match self.cat {
            OrdCategory::F | OrdCategory::G => (Stretchy::Always, false),
            OrdCategory::FGandForceDefault => (Stretchy::PrePostfix, true),
            OrdCategory::K => (Stretchy::Never, false),
            OrdCategory::KButUsedToBeB => (Stretchy::Never, true),
            _ => {
                return None;
            }
        };
        Some(StretchableOp {
            char: self.char,
            stretchy,
            nonzero_spacing,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCategory {
    /// Category C: Infix, op spacing (e.g. `×`).
    C,
    /// Category H: Prefix, op spacing, symmetric, largeop (e.g. `∫`).
    H,
    /// Category J: Prefix, op spacing, symmetric, largeop, movablelimits
    /// (e.g. `∑`).
    J,
}

make_character_class!(
    /// An operator with operator spacing (categories C, H, J).
    Op, OpCategory
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinCategory {
    /// Category B: Infix, binary op spacing (e.g. `÷`)
    B,
    /// Category B and D: Infix and prefix, binary op spacing (e.g. `+`)
    BD,
}

make_character_class!(
    /// An operator with binary operator spacing (categories B and D).
    Bin, BinCategory
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelCategory {
    /// Default: No form, relation spacing (e.g. `=`).
    Default,
    /// Category A: Infix, relation spacing, stretchy (e.g. `↑`).
    A,
}

make_character_class!(
    /// An operator with relation spacing.
    Rel, RelCategory
);

impl Rel {
    #[inline(always)]
    pub const fn as_stretchable_op(&self) -> Option<StretchableOp> {
        match self.cat {
            RelCategory::A => Some(StretchableOp {
                char: self.char,
                stretchy: Stretchy::AlwaysAsymmetric,
                nonzero_spacing: true,
            }),
            _ => None,
        }
    }
}

/// An operator with punctuation spacing (category M).
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(transparent)]
pub struct Punct(char);

impl Punct {
    #[inline(always)]
    pub const fn as_op(&self) -> MathMLOperator {
        MathMLOperator(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Stretchy {
    /// The operator is always stretchy (e.g. `(`, `)`).
    Always = 1,
    /// The operator is only stretchy as a pre- or postfix operator (e.g. `|`).
    PrePostfix,
    /// The operator is never stretchy (e.g. `/`).
    Never,
    /// The operator is always stretchy but isn't symmetric (e.g. `↑`).
    AlwaysAsymmetric,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct StretchableOp {
    char: BMPChar,
    pub stretchy: Stretchy,
    pub nonzero_spacing: bool,
}

#[cfg(feature = "serde")]
impl Serialize for StretchableOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTupleStruct;

        let mut state = serializer.serialize_tuple_struct("StretchableOp", 2)?;
        state.serialize_field(&char::from(*self))?;
        state.serialize_field(&self.stretchy)?;
        state.end()
    }
}

impl From<StretchableOp> for char {
    #[inline]
    fn from(op: StretchableOp) -> Self {
        op.char.as_char()
    }
}

//
// Unicode Block: Basic Latin
//
pub const EXCLAMATION_MARK: MathMLOperator = MathMLOperator('!');
// pub const QUOTATION_MARK: char = '"';
pub const NUMBER_SIGN: char = '#';
pub const DOLLAR_SIGN: char = '$';
pub const PERCENT_SIGN: char = '%';
pub const AMPERSAND: OrdLike = OrdLike::new('&', OrdCategory::E);
// pub const APOSTROPHE: char = '\'';
pub const LEFT_PARENTHESIS: OrdLike = OrdLike::new('(', OrdCategory::F);
pub const RIGHT_PARENTHESIS: OrdLike = OrdLike::new(')', OrdCategory::G);
// pub const ASTERISK: char = '*';
pub const PLUS_SIGN: Bin = Bin::new('+', BinCategory::BD);
pub const COMMA: Punct = Punct(',');
pub const FULL_STOP: Op = Op::new('.', OpCategory::C);
pub const SOLIDUS: OrdLike = OrdLike::new('/', OrdCategory::KButUsedToBeB);

pub const COLON: Punct = Punct(':');
pub const SEMICOLON: Punct = Punct(';');
// pub const LESS_THAN_SIGN: char = '<';
pub const EQUALS_SIGN: Rel = Rel::new('=', RelCategory::Default);
// pub const GREATER_THAN_SIGN: char = '>';
// pub const QUESTION_MARK: char = '?';
// pub const COMMERCIAL_AT: char = '@';

pub const LEFT_SQUARE_BRACKET: OrdLike = OrdLike::new('[', OrdCategory::F);
pub const REVERSE_SOLIDUS: OrdLike = OrdLike::new('\\', OrdCategory::K);
pub const RIGHT_SQUARE_BRACKET: OrdLike = OrdLike::new(']', OrdCategory::G);
// pub const CIRCUMFLEX_ACCENT: Rel = Rel::new('^', RelCategory::Default);
pub const LOW_LINE: Rel = Rel::new('_', RelCategory::Default);
pub const GRAVE_ACCENT: Rel = Rel::new('`', RelCategory::Default);

pub const LEFT_CURLY_BRACKET: OrdLike = OrdLike::new('{', OrdCategory::F);
pub const VERTICAL_LINE: OrdLike = OrdLike::new('|', OrdCategory::FGandForceDefault);
pub const RIGHT_CURLY_BRACKET: OrdLike = OrdLike::new('}', OrdCategory::G);
pub const TILDE: Rel = Rel::new('~', RelCategory::Default);

//
// Unicode Block: Latin-1 Supplement
//
pub const SECTION_SIGN: char = '§';
pub const DIAERESIS: Rel = Rel::new('¨', RelCategory::Default);
pub const COPYRIGHT_SIGN: char = '©';

pub const NOT_SIGN: OrdLike = OrdLike::new('¬', OrdCategory::D);

pub const MACRON: Rel = Rel::new('¯', RelCategory::Default);

pub const PLUS_MINUS_SIGN: Bin = Bin::new('±', BinCategory::BD);

pub const ACUTE_ACCENT: Rel = Rel::new('´', RelCategory::Default);

pub const PILCROW_SIGN: char = '¶';
pub const MIDDLE_DOT: Op = Op::new('·', OpCategory::C);

pub const MULTIPLICATION_SIGN: Op = Op::new('×', OpCategory::C);

pub const LATIN_SMALL_LETTER_ETH: char = 'ð';

pub const DIVISION_SIGN: Bin = Bin::new('÷', BinCategory::BD);

pub const LATIN_SMALL_LETTER_THORN: char = 'þ';

//
// Unicode Block: Latin Extended-A
//
pub const LATIN_SMALL_LETTER_DOTLESS_I: char = 'ı';

//
// Unicode Block: Latin Extended-B
//
pub const LATIN_SMALL_LETTER_DOTLESS_J: char = 'ȷ';

//
// Unicode Block: Spacing Modifier Letters
//
pub const CARON: Rel = Rel::new('ˇ', RelCategory::Default);
pub const BREVE: Rel = Rel::new('˘', RelCategory::Default);
pub const DOT_ABOVE: Rel = Rel::new('˙', RelCategory::Default);

//
// Unicode Block: Combining Diacritical Marks
//
pub const COMBINING_GRAVE_ACCENT: char = '\u{300}';
pub const COMBINING_ACUTE_ACCENT: char = '\u{301}';
pub const COMBINING_CIRCUMFLEX_ACCENT: Rel = Rel::new('\u{302}', RelCategory::Default);
pub const COMBINING_TILDE: Rel = Rel::new('\u{303}', RelCategory::Default);
// pub const COMBINING_MACRON: char = '\u{304}';
pub const COMBINING_OVERLINE: char = '\u{305}';
pub const COMBINING_BREVE: char = '\u{306}';
pub const COMBINING_DOT_ABOVE: char = '\u{307}';
pub const COMBINING_DIAERESIS: char = '\u{308}';
// pub const COMBINING_HOOK_ABOVE: char = '\u{309}';
pub const COMBINING_RING_ABOVE: char = '\u{30A}';
pub const COMBINING_DOUBLE_ACUTE_ACCENT: char = '\u{30B}';
pub const COMBINING_CARON: Rel = Rel::new('\u{30C}', RelCategory::Default);

pub const COMBINING_CEDILLA: char = '\u{327}';

//
// Unicode Block: Greek and Coptic
//
pub const GREEK_CAPITAL_LETTER_ALPHA: char = 'Α';
pub const GREEK_CAPITAL_LETTER_BETA: char = 'Β';
pub const GREEK_CAPITAL_LETTER_GAMMA: char = 'Γ';
pub const GREEK_CAPITAL_LETTER_DELTA: char = 'Δ';
pub const GREEK_CAPITAL_LETTER_EPSILON: char = 'Ε';
pub const GREEK_CAPITAL_LETTER_ZETA: char = 'Ζ';
pub const GREEK_CAPITAL_LETTER_ETA: char = 'Η';
pub const GREEK_CAPITAL_LETTER_THETA: char = 'Θ';
pub const GREEK_CAPITAL_LETTER_IOTA: char = 'Ι';
pub const GREEK_CAPITAL_LETTER_KAPPA: char = 'Κ';
pub const GREEK_CAPITAL_LETTER_LAMBDA: char = 'Λ';
pub const GREEK_CAPITAL_LETTER_MU: char = 'Μ';
pub const GREEK_CAPITAL_LETTER_NU: char = 'Ν';
pub const GREEK_CAPITAL_LETTER_XI: char = 'Ξ';
pub const GREEK_CAPITAL_LETTER_OMICRON: char = 'Ο';
pub const GREEK_CAPITAL_LETTER_PI: char = 'Π';
pub const GREEK_CAPITAL_LETTER_RHO: char = 'Ρ';
pub const GREEK_CAPITAL_LETTER_SIGMA: char = 'Σ';
pub const GREEK_CAPITAL_LETTER_TAU: char = 'Τ';
pub const GREEK_CAPITAL_LETTER_UPSILON: char = 'Υ';
pub const GREEK_CAPITAL_LETTER_PHI: char = 'Φ';
pub const GREEK_CAPITAL_LETTER_CHI: char = 'Χ';
pub const GREEK_CAPITAL_LETTER_PSI: char = 'Ψ';
pub const GREEK_CAPITAL_LETTER_OMEGA: char = 'Ω';

pub const GREEK_SMALL_LETTER_ALPHA: char = 'α';
pub const GREEK_SMALL_LETTER_BETA: char = 'β';
pub const GREEK_SMALL_LETTER_GAMMA: char = 'γ';
pub const GREEK_SMALL_LETTER_DELTA: char = 'δ';
pub const GREEK_SMALL_LETTER_EPSILON: char = 'ε';
pub const GREEK_SMALL_LETTER_ZETA: char = 'ζ';
pub const GREEK_SMALL_LETTER_ETA: char = 'η';
pub const GREEK_SMALL_LETTER_THETA: char = 'θ';
pub const GREEK_SMALL_LETTER_IOTA: char = 'ι';
pub const GREEK_SMALL_LETTER_KAPPA: char = 'κ';
pub const GREEK_SMALL_LETTER_LAMBDA: char = 'λ';
pub const GREEK_SMALL_LETTER_MU: char = 'μ';
pub const GREEK_SMALL_LETTER_NU: char = 'ν';
pub const GREEK_SMALL_LETTER_XI: char = 'ξ';
pub const GREEK_SMALL_LETTER_OMICRON: char = 'ο';
pub const GREEK_SMALL_LETTER_PI: char = 'π';
pub const GREEK_SMALL_LETTER_RHO: char = 'ρ';
pub const GREEK_SMALL_LETTER_FINAL_SIGMA: char = 'ς';
pub const GREEK_SMALL_LETTER_SIGMA: char = 'σ';
pub const GREEK_SMALL_LETTER_TAU: char = 'τ';
pub const GREEK_SMALL_LETTER_UPSILON: char = 'υ';
pub const GREEK_SMALL_LETTER_PHI: char = 'φ';
pub const GREEK_SMALL_LETTER_CHI: char = 'χ';
pub const GREEK_SMALL_LETTER_PSI: char = 'ψ';
pub const GREEK_SMALL_LETTER_OMEGA: char = 'ω';

pub const GREEK_THETA_SYMBOL: char = 'ϑ';
// pub const GREEK_UPSILON_WITH_HOOK_SYMBOL: char = 'ϒ';

pub const GREEK_PHI_SYMBOL: char = 'ϕ';
pub const GREEK_PI_SYMBOL: char = 'ϖ';

// pub const GREEK_LETTER_DIGAMMA: char = 'Ϝ';
pub const GREEK_SMALL_LETTER_DIGAMMA: char = 'ϝ';

pub const GREEK_KAPPA_SYMBOL: char = 'ϰ';
pub const GREEK_RHO_SYMBOL: char = 'ϱ';

// pub const GREEK_CAPITAL_THETA_SYMBOL: char = 'ϴ';

pub const GREEK_LUNATE_EPSILON_SYMBOL: char = 'ϵ';
pub const GREEK_REVERSED_LUNATE_EPSILON_SYMBOL: char = '϶';

//
// Unicode Block: General Punctuation
//
pub const DOUBLE_VERTICAL_LINE: OrdLike = OrdLike::new('‖', OrdCategory::FGandForceDefault); // should actually be FG

pub const DAGGER: char = '†';
pub const DOUBLE_DAGGER: char = '‡';

pub const HORIZONTAL_ELLIPSIS: char = '…';
pub const PRIME: OrdLike = OrdLike::new('′', OrdCategory::E);
pub const DOUBLE_PRIME: OrdLike = OrdLike::new('″', OrdCategory::E);
pub const TRIPLE_PRIME: OrdLike = OrdLike::new('‴', OrdCategory::E);
pub const REVERSED_PRIME: OrdLike = OrdLike::new('‵', OrdCategory::E);
pub const REVERSED_DOUBLE_PRIME: OrdLike = OrdLike::new('‶', OrdCategory::E);
pub const REVERSED_TRIPLE_PRIME: OrdLike = OrdLike::new('‷', OrdCategory::E);
// pub const CARET: Ord = ord('‸');
// pub const SINGLE_LEFT_POINTING_ANGLE_QUOTATION_MARK: Ord = ord('‹');
// pub const SINGLE_RIGHT_POINTING_ANGLE_QUOTATION_MARK: Ord = ord('›');
// pub const REFERENCE_MARK: Ord = ord('※');
// pub const DOUBLE_EXCLAMATION_MARK: Ord = ord('‼');
// pub const INTERROBANG: Ord = ord('‽');
pub const OVERLINE: Rel = Rel::new('‾', RelCategory::Default);

pub const QUADRUPLE_PRIME: OrdLike = OrdLike::new('⁗', OrdCategory::E);

//
// Unicode Block: Combining Diacritical Marks for Symbols
//
pub const COMBINING_RIGHT_ARROW_ABOVE: Rel = Rel::new('\u{20D7}', RelCategory::Default);

pub const COMBINING_THREE_DOTS_ABOVE: Rel = Rel::new('\u{20DB}', RelCategory::Default);
pub const COMBINING_FOUR_DOTS_ABOVE: Rel = Rel::new('\u{20DC}', RelCategory::Default);

//
// Unicode Block: Letterlike Symbols
//
pub const PLANCK_CONSTANT_OVER_TWO_PI: char = 'ℏ';
// pub const SCRIPT_CAPITAL_I: char = 'ℐ';
pub const BLACK_LETTER_CAPITAL_I: char = 'ℑ';
// pub const SCRIPT_CAPITAL_L: char = 'ℒ';
pub const SCRIPT_SMALL_L: char = 'ℓ';

pub const SCRIPT_CAPITAL_P: char = '℘';

pub const BLACK_LETTER_CAPITAL_R: char = 'ℜ';
pub const DOUBLE_STRUCK_CAPITAL_R: char = 'ℝ';

pub const INVERTED_OHM_SIGN: char = '℧';

pub const ANGSTROM_SIGN: char = 'Å';

pub const TURNED_CAPITAL_F: char = 'Ⅎ';

pub const ALEF_SYMBOL: char = 'ℵ';
pub const BET_SYMBOL: char = 'ℶ';
pub const GIMEL_SYMBOL: char = 'ℷ';
pub const DALET_SYMBOL: char = 'ℸ';

pub const TURNED_SANS_SERIF_CAPITAL_G: char = '⅁';

//
// Unicode Block: Arrows
//
pub const LEFTWARDS_ARROW: Rel = Rel::new('←', RelCategory::Default);
pub const UPWARDS_ARROW: Rel = Rel::new('↑', RelCategory::A);
pub const RIGHTWARDS_ARROW: Rel = Rel::new('→', RelCategory::Default);
pub const DOWNWARDS_ARROW: Rel = Rel::new('↓', RelCategory::A);
pub const LEFT_RIGHT_ARROW: Rel = Rel::new('↔', RelCategory::Default);
pub const UP_DOWN_ARROW: Rel = Rel::new('↕', RelCategory::A);
pub const NORTH_WEST_ARROW: Rel = Rel::new('↖', RelCategory::Default);
pub const NORTH_EAST_ARROW: Rel = Rel::new('↗', RelCategory::Default);
pub const SOUTH_EAST_ARROW: Rel = Rel::new('↘', RelCategory::Default);
pub const SOUTH_WEST_ARROW: Rel = Rel::new('↙', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_STROKE: Rel = Rel::new('↚', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_STROKE: Rel = Rel::new('↛', RelCategory::Default);
// pub const LEFTWARDS_WAVE_ARROW: Rel = Rel::new('↜', RelCategory::Default);
// pub const RIGHTWARDS_WAVE_ARROW: Rel = Rel::new('↝', RelCategory::Default);
pub const LEFTWARDS_TWO_HEADED_ARROW: Rel = Rel::new('↞', RelCategory::Default);
// pub const UPWARDS_TWO_HEADED_ARROW: Rel = Rel::new('↟', RelCategory::Default);
pub const RIGHTWARDS_TWO_HEADED_ARROW: Rel = Rel::new('↠', RelCategory::Default);
// pub const DOWNWARDS_TWO_HEADED_ARROW: Rel = Rel::new('↡', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_TAIL: Rel = Rel::new('↢', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_TAIL: Rel = Rel::new('↣', RelCategory::Default);
// pub const LEFTWARDS_ARROW_FROM_BAR: Rel = Rel::new('↤', RelCategory::Default);
// pub const UPWARDS_ARROW_FROM_BAR: Rel = Rel::new('↥', RelCategory::Default);
pub const RIGHTWARDS_ARROW_FROM_BAR: Rel = Rel::new('↦', RelCategory::Default);
// pub const DOWNWARDS_ARROW_FROM_BAR: Rel = Rel::new('↧', RelCategory::Default);
// pub const UP_DOWN_ARROW_WITH_BASE: Rel = Rel::new('↨', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_HOOK: Rel = Rel::new('↩', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_HOOK: Rel = Rel::new('↪', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_LOOP: Rel = Rel::new('↫', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_LOOP: Rel = Rel::new('↬', RelCategory::Default);
pub const LEFT_RIGHT_WAVE_ARROW: Rel = Rel::new('↭', RelCategory::Default);
pub const LEFT_RIGHT_ARROW_WITH_STROKE: Rel = Rel::new('↮', RelCategory::Default);
pub const DOWNWARDS_ZIGZAG_ARROW: Rel = Rel::new('↯', RelCategory::Default);
pub const UPWARDS_ARROW_WITH_TIP_LEFTWARDS: Rel = Rel::new('↰', RelCategory::Default);
pub const UPWARDS_ARROW_WITH_TIP_RIGHTWARDS: Rel = Rel::new('↱', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_TIP_LEFTWARDS: Rel = Rel::new('↲', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_TIP_RIGHTWARDS: Rel = Rel::new('↳', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_CORNER_DOWNWARDS: Rel = Rel::new('↴', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_CORNER_LEFTWARDS: Rel = Rel::new('↵', RelCategory::Default);
pub const ANTICLOCKWISE_TOP_SEMICIRCLE_ARROW: Rel = Rel::new('↶', RelCategory::Default);
pub const CLOCKWISE_TOP_SEMICIRCLE_ARROW: Rel = Rel::new('↷', RelCategory::Default);
// pub const NORTH_WEST_ARROW_TO_LONG_BAR: Rel = Rel::new('↸', RelCategory::Default);
// pub const LEFTWARDS_ARROW_TO_BAR_OVER_RIGHTWARDS_ARROW_TO_BAR: Rel = Rel::new('↹', RelCategory::Default);
pub const ANTICLOCKWISE_OPEN_CIRCLE_ARROW: Rel = Rel::new('↺', RelCategory::Default);
pub const CLOCKWISE_OPEN_CIRCLE_ARROW: Rel = Rel::new('↻', RelCategory::Default);
pub const LEFTWARDS_HARPOON_WITH_BARB_UPWARDS: Rel = Rel::new('↼', RelCategory::Default);
pub const LEFTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Rel = Rel::new('↽', RelCategory::Default);
pub const UPWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Rel = Rel::new('↾', RelCategory::Default);
pub const UPWARDS_HARPOON_WITH_BARB_LEFTWARDS: Rel = Rel::new('↿', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_WITH_BARB_UPWARDS: Rel = Rel::new('⇀', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Rel = Rel::new('⇁', RelCategory::Default);
pub const DOWNWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Rel = Rel::new('⇂', RelCategory::Default);
pub const DOWNWARDS_HARPOON_WITH_BARB_LEFTWARDS: Rel = Rel::new('⇃', RelCategory::Default);
pub const RIGHTWARDS_ARROW_OVER_LEFTWARDS_ARROW: Rel = Rel::new('⇄', RelCategory::Default);
// pub const UPWARDS_ARROW_LEFTWARDS_OF_DOWNWARDS_ARROW: Rel = Rel::new('⇅', RelCategory::Default);
pub const LEFTWARDS_ARROW_OVER_RIGHTWARDS_ARROW: Rel = Rel::new('⇆', RelCategory::Default);
pub const LEFTWARDS_PAIRED_ARROWS: Rel = Rel::new('⇇', RelCategory::Default);
pub const UPWARDS_PAIRED_ARROWS: Rel = Rel::new('⇈', RelCategory::Default);
pub const RIGHTWARDS_PAIRED_ARROWS: Rel = Rel::new('⇉', RelCategory::Default);
pub const DOWNWARDS_PAIRED_ARROWS: Rel = Rel::new('⇊', RelCategory::Default);
pub const LEFTWARDS_HARPOON_OVER_RIGHTWARDS_HARPOON: Rel = Rel::new('⇋', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_OVER_LEFTWARDS_HARPOON: Rel = Rel::new('⇌', RelCategory::Default);
pub const LEFTWARDS_DOUBLE_ARROW_WITH_STROKE: Rel = Rel::new('⇍', RelCategory::Default);
pub const LEFT_RIGHT_DOUBLE_ARROW_WITH_STROKE: Rel = Rel::new('⇎', RelCategory::Default);
pub const RIGHTWARDS_DOUBLE_ARROW_WITH_STROKE: Rel = Rel::new('⇏', RelCategory::Default);
pub const LEFTWARDS_DOUBLE_ARROW: Rel = Rel::new('⇐', RelCategory::Default);
pub const UPWARDS_DOUBLE_ARROW: Rel = Rel::new('⇑', RelCategory::A);
pub const RIGHTWARDS_DOUBLE_ARROW: Rel = Rel::new('⇒', RelCategory::Default);
pub const DOWNWARDS_DOUBLE_ARROW: Rel = Rel::new('⇓', RelCategory::A);
pub const LEFT_RIGHT_DOUBLE_ARROW: Rel = Rel::new('⇔', RelCategory::Default);
pub const UP_DOWN_DOUBLE_ARROW: Rel = Rel::new('⇕', RelCategory::A);
// pub const NORTH_WEST_DOUBLE_ARROW: Rel = Rel::new('⇖', RelCategory::Default);
// pub const NORTH_EAST_DOUBLE_ARROW: Rel = Rel::new('⇗', RelCategory::Default);
// pub const SOUTH_EAST_DOUBLE_ARROW: Rel = Rel::new('⇘', RelCategory::Default);
// pub const SOUTH_WEST_DOUBLE_ARROW: Rel = Rel::new('⇙', RelCategory::Default);
pub const LEFTWARDS_TRIPLE_ARROW: Rel = Rel::new('⇚', RelCategory::Default);
pub const RIGHTWARDS_TRIPLE_ARROW: Rel = Rel::new('⇛', RelCategory::Default);
// pub const LEFTWARDS_SQUIGGLE_ARROW: Rel = Rel::new('⇜', RelCategory::Default);
pub const RIGHTWARDS_SQUIGGLE_ARROW: Rel = Rel::new('⇝', RelCategory::Default);
// pub const UPWARDS_ARROW_WITH_DOUBLE_STROKE: Rel = Rel::new('⇞', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_DOUBLE_STROKE: Rel = Rel::new('⇟', RelCategory::Default);
// pub const LEFTWARDS_DASHED_ARROW: Rel = Rel::new('⇠', RelCategory::Default);
// pub const UPWARDS_DASHED_ARROW: Rel = Rel::new('⇡', RelCategory::Default);
// pub const RIGHTWARDS_DASHED_ARROW: Rel = Rel::new('⇢', RelCategory::Default);
// pub const DOWNWARDS_DASHED_ARROW: Rel = Rel::new('⇣', RelCategory::Default);
// pub const LEFTWARDS_ARROW_TO_BAR: Rel = Rel::new('⇤', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_TO_BAR: Rel = Rel::new('⇥', RelCategory::Default);
// pub const LEFTWARDS_WHITE_ARROW: Rel = Rel::new('⇦', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW: Rel = Rel::new('⇧', RelCategory::Default);
// pub const RIGHTWARDS_WHITE_ARROW: Rel = Rel::new('⇨', RelCategory::Default);
// pub const DOWNWARDS_WHITE_ARROW: Rel = Rel::new('⇩', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_FROM_BAR: Rel = Rel::new('⇪', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL: Rel = Rel::new('⇫', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_HORIZONTAL_BAR: Rel = Rel::new('⇬', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_VERTICAL_BAR: Rel = Rel::new('⇭', RelCategory::Default);
// pub const UPWARDS_WHITE_DOUBLE_ARROW: Rel = Rel::new('⇮', RelCategory::Default);
// pub const UPWARDS_WHITE_DOUBLE_ARROW_ON_PEDESTAL: Rel = Rel::new('⇯', RelCategory::Default);
// pub const RIGHTWARDS_WHITE_ARROW_FROM_WALL: Rel = Rel::new('⇰', RelCategory::Default);
// pub const NORTH_WEST_ARROW_TO_CORNER: Rel = Rel::new('⇱', RelCategory::Default);
// pub const SOUTH_EAST_ARROW_TO_CORNER: Rel = Rel::new('⇲', RelCategory::Default);
// pub const UP_DOWN_WHITE_ARROW: Rel = Rel::new('⇳', RelCategory::Default);
// pub const RIGHT_ARROW_WITH_SMALL_CIRCLE: Rel = Rel::new('⇴', RelCategory::Default);
// pub const DOWNWARDS_ARROW_LEFTWARDS_OF_UPWARDS_ARROW: Rel = Rel::new('⇵', RelCategory::Default);
// pub const THREE_RIGHTWARDS_ARROWS: Rel = Rel::new('⇶', RelCategory::Default);
// pub const LEFTWARDS_ARROW_WITH_VERTICAL_STROKE: Rel = Rel::new('⇷', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_VERTICAL_STROKE: Rel = Rel::new('⇸', RelCategory::Default);
// pub const LEFT_RIGHT_ARROW_WITH_VERTICAL_STROKE: Rel = Rel::new('⇹', RelCategory::Default);
// pub const LEFTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = Rel::new('⇺', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = Rel::new('⇻', RelCategory::Default);
// pub const LEFT_RIGHT_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = Rel::new('⇼', RelCategory::Default);
// pub const LEFTWARDS_OPEN_HEADED_ARROW: Rel = Rel::new('⇽', RelCategory::Default);
// pub const RIGHTWARDS_OPEN_HEADED_ARROW: Rel = Rel::new('⇾', RelCategory::Default);
// pub const LEFT_RIGHT_OPEN_HEADED_ARROW: Rel = Rel::new('⇿', RelCategory::Default);

//
// Unicode Block: Mathematical Operators
//
pub const FOR_ALL: OrdLike = OrdLike::new('∀', OrdCategory::D);
pub const COMPLEMENT: OrdLike = OrdLike::new('∁', OrdCategory::D);
pub const PARTIAL_DIFFERENTIAL: char = '∂'; // char so that it can be transformed
pub const THERE_EXISTS: OrdLike = OrdLike::new('∃', OrdCategory::D);
pub const THERE_DOES_NOT_EXIST: OrdLike = OrdLike::new('∄', OrdCategory::D);
pub const EMPTY_SET: char = '∅';
// pub const INCREMENT: Ord = ord('∆');
pub const NABLA: char = '∇'; // char so that it can be transformed
pub const ELEMENT_OF: Rel = Rel::new('∈', RelCategory::Default);
pub const NOT_AN_ELEMENT_OF: Rel = Rel::new('∉', RelCategory::Default);
// pub const SMALL_ELEMENT_OF: Rel = Rel::new('∊', RelCategory::Default);
pub const CONTAINS_AS_MEMBER: Rel = Rel::new('∋', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_MEMBER: Rel = Rel::new('∌', RelCategory::Default);
// pub const SMALL_CONTAINS_AS_MEMBER: Rel = Rel::new('∍', RelCategory::Default);
// pub const END_OF_PROOF: Ord = ord('∎');
pub const N_ARY_PRODUCT: Op = Op::new('∏', OpCategory::J);
pub const N_ARY_COPRODUCT: Op = Op::new('∐', OpCategory::J);
pub const N_ARY_SUMMATION: Op = Op::new('∑', OpCategory::J);
pub const MINUS_SIGN: Bin = Bin::new('−', BinCategory::BD);
pub const MINUS_OR_PLUS_SIGN: Bin = Bin::new('∓', BinCategory::BD);
pub const DOT_PLUS: Bin = Bin::new('∔', BinCategory::BD);
// pub const DIVISION_SLASH: Op = Op('∕');
pub const SET_MINUS: Bin = Bin::new('∖', BinCategory::BD);
pub const ASTERISK_OPERATOR: Bin = Bin::new('∗', BinCategory::BD);
pub const RING_OPERATOR: Op = Op::new('∘', OpCategory::C);
pub const BULLET_OPERATOR: Op = Op::new('∙', OpCategory::C);
// pub const SQUARE_ROOT: Op = Op('√');
// pub const CUBE_ROOT: Op = Op('∛');
// pub const FOURTH_ROOT: Op = Op('∜');
pub const PROPORTIONAL_TO: Rel = Rel::new('∝', RelCategory::Default);
pub const INFINITY: char = '∞';
// pub const RIGHT_ANGLE: Op = Op('∟');
pub const ANGLE: char = '∠';
pub const MEASURED_ANGLE: char = '∡';
pub const SPHERICAL_ANGLE: char = '∢';
pub const DIVIDES: Rel = Rel::new('∣', RelCategory::Default);
pub const DOES_NOT_DIVIDE: Rel = Rel::new('∤', RelCategory::Default);
pub const PARALLEL_TO: Rel = Rel::new('∥', RelCategory::Default);
pub const NOT_PARALLEL_TO: Rel = Rel::new('∦', RelCategory::Default);
pub const LOGICAL_AND: Bin = Bin::new('∧', BinCategory::BD);
pub const LOGICAL_OR: Bin = Bin::new('∨', BinCategory::BD);
pub const INTERSECTION: Bin = Bin::new('∩', BinCategory::BD);
pub const UNION: Bin = Bin::new('∪', BinCategory::BD);
pub const INTEGRAL: Op = Op::new('∫', OpCategory::H);
pub const DOUBLE_INTEGRAL: Op = Op::new('∬', OpCategory::H);
pub const TRIPLE_INTEGRAL: Op = Op::new('∭', OpCategory::H);
pub const CONTOUR_INTEGRAL: Op = Op::new('∮', OpCategory::H);
pub const SURFACE_INTEGRAL: Op = Op::new('∯', OpCategory::H);
pub const VOLUME_INTEGRAL: Op = Op::new('∰', OpCategory::H);
pub const CLOCKWISE_INTEGRAL: Op = Op::new('∱', OpCategory::H);
pub const CLOCKWISE_CONTOUR_INTEGRAL: Op = Op::new('∲', OpCategory::H);
pub const ANTICLOCKWISE_CONTOUR_INTEGRAL: Op = Op::new('∳', OpCategory::H);
pub const THEREFORE: Rel = Rel::new('∴', RelCategory::Default);
pub const BECAUSE: Rel = Rel::new('∵', RelCategory::Default);
// pub const RATIO: Op = Op('∶');
pub const PROPORTION: Rel = Rel::new('∷', RelCategory::Default);
// pub const DOT_MINUS: Op = Op('∸');
pub const EXCESS: Rel = Rel::new('∹', RelCategory::Default);
pub const GEOMETRIC_PROPORTION: Rel = Rel::new('∺', RelCategory::Default);
pub const HOMOTHETIC: Rel = Rel::new('∻', RelCategory::Default);
pub const TILDE_OPERATOR: Rel = Rel::new('∼', RelCategory::Default);
pub const REVERSED_TILDE: Rel = Rel::new('∽', RelCategory::Default);
// pub const INVERTED_LAZY_S: Op = Op('∾');
// pub const SINE_WAVE: Op = Op('∿');
pub const WREATH_PRODUCT: Op = Op::new('≀', OpCategory::C);
pub const NOT_TILDE: Rel = Rel::new('≁', RelCategory::Default);
pub const MINUS_TILDE: Rel = Rel::new('≂', RelCategory::Default);
pub const ASYMPTOTICALLY_EQUAL_TO: Rel = Rel::new('≃', RelCategory::Default);
pub const NOT_ASYMPTOTICALLY_EQUAL_TO: Rel = Rel::new('≄', RelCategory::Default);
pub const APPROXIMATELY_EQUAL_TO: Rel = Rel::new('≅', RelCategory::Default);
// pub const APPROXIMATELY_BUT_NOT_ACTUALLY_EQUAL_TO: Rel = Rel::new('≆', RelCategory::Default);
// pub const NEITHER_APPROXIMATELY_NOR_ACTUALLY_EQUAL_TO: Rel = Rel::new('≇', RelCategory::Default);
pub const ALMOST_EQUAL_TO: Rel = Rel::new('≈', RelCategory::Default);
pub const NOT_ALMOST_EQUAL_TO: Rel = Rel::new('≉', RelCategory::Default);
pub const ALMOST_EQUAL_OR_EQUAL_TO: Rel = Rel::new('≊', RelCategory::Default);
// pub const TRIPLE_TILDE: Rel = Rel::new('≋', RelCategory::Default);
// pub const ALL_EQUAL_TO: Rel = Rel::new('≌', RelCategory::Default);
pub const EQUIVALENT_TO: Rel = Rel::new('≍', RelCategory::Default);
pub const GEOMETRICALLY_EQUIVALENT_TO: Rel = Rel::new('≎', RelCategory::Default);
pub const DIFFERENCE_BETWEEN: Rel = Rel::new('≏', RelCategory::Default);
pub const APPROACHES_THE_LIMIT: Rel = Rel::new('≐', RelCategory::Default);
pub const GEOMETRICALLY_EQUAL_TO: Rel = Rel::new('≑', RelCategory::Default);
pub const APPROXIMATELY_EQUAL_TO_OR_THE_IMAGE_OF: Rel = Rel::new('≒', RelCategory::Default);
pub const IMAGE_OF_OR_APPROXIMATELY_EQUAL_TO: Rel = Rel::new('≓', RelCategory::Default);
pub const COLON_EQUALS: Rel = Rel::new('≔', RelCategory::Default);
pub const EQUALS_COLON: Rel = Rel::new('≕', RelCategory::Default);
pub const RING_IN_EQUAL_TO: Rel = Rel::new('≖', RelCategory::Default);
pub const RING_EQUAL_TO: Rel = Rel::new('≗', RelCategory::Default);
pub const CORRESPONDS_TO: Rel = Rel::new('≘', RelCategory::Default);
pub const ESTIMATES: Rel = Rel::new('≙', RelCategory::Default);
pub const EQUIANGULAR_TO: Rel = Rel::new('≚', RelCategory::Default);
pub const STAR_EQUALS: Rel = Rel::new('≛', RelCategory::Default);
pub const DELTA_EQUAL_TO: Rel = Rel::new('≜', RelCategory::Default);
pub const EQUAL_TO_BY_DEFINITION: Rel = Rel::new('≝', RelCategory::Default);
pub const MEASURED_BY: Rel = Rel::new('≞', RelCategory::Default);
pub const QUESTIONED_EQUAL_TO: Rel = Rel::new('≟', RelCategory::Default);
pub const NOT_EQUAL_TO: Rel = Rel::new('≠', RelCategory::Default);
pub const IDENTICAL_TO: Rel = Rel::new('≡', RelCategory::Default);
pub const NOT_IDENTICAL_TO: Rel = Rel::new('≢', RelCategory::Default);
// pub const STRICTLY_EQUIVALENT_TO: Rel = Rel::new('≣', RelCategory::Default);
pub const LESS_THAN_OR_EQUAL_TO: Rel = Rel::new('≤', RelCategory::Default);
pub const GREATER_THAN_OR_EQUAL_TO: Rel = Rel::new('≥', RelCategory::Default);
pub const LESS_THAN_OVER_EQUAL_TO: Rel = Rel::new('≦', RelCategory::Default);
pub const GREATER_THAN_OVER_EQUAL_TO: Rel = Rel::new('≧', RelCategory::Default);
pub const LESS_THAN_BUT_NOT_EQUAL_TO: Rel = Rel::new('≨', RelCategory::Default);
pub const GREATER_THAN_BUT_NOT_EQUAL_TO: Rel = Rel::new('≩', RelCategory::Default);
pub const MUCH_LESS_THAN: Rel = Rel::new('≪', RelCategory::Default);
pub const MUCH_GREATER_THAN: Rel = Rel::new('≫', RelCategory::Default);
pub const BETWEEN: Rel = Rel::new('≬', RelCategory::Default);
// pub const NOT_EQUIVALENT_TO: Rel = Rel::new('≭', RelCategory::Default);
pub const NOT_LESS_THAN: Rel = Rel::new('≮', RelCategory::Default);
pub const NOT_GREATER_THAN: Rel = Rel::new('≯', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_EQUAL_TO: Rel = Rel::new('≰', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_EQUAL_TO: Rel = Rel::new('≱', RelCategory::Default);
pub const LESS_THAN_OR_EQUIVALENT_TO: Rel = Rel::new('≲', RelCategory::Default);
pub const GREATER_THAN_OR_EQUIVALENT_TO: Rel = Rel::new('≳', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_EQUIVALENT_TO: Rel = Rel::new('≴', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_EQUIVALENT_TO: Rel = Rel::new('≵', RelCategory::Default);
pub const LESS_THAN_OR_GREATER_THAN: Rel = Rel::new('≶', RelCategory::Default);
pub const GREATER_THAN_OR_LESS_THAN: Rel = Rel::new('≷', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_GREATER_THAN: Rel = Rel::new('≸', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_LESS_THAN: Rel = Rel::new('≹', RelCategory::Default);
pub const PRECEDES: Rel = Rel::new('≺', RelCategory::Default);
pub const SUCCEEDS: Rel = Rel::new('≻', RelCategory::Default);
pub const PRECEDES_OR_EQUAL_TO: Rel = Rel::new('≼', RelCategory::Default);
pub const SUCCEEDS_OR_EQUAL_TO: Rel = Rel::new('≽', RelCategory::Default);
pub const PRECEDES_OR_EQUIVALENT_TO: Rel = Rel::new('≾', RelCategory::Default);
pub const SUCCEEDS_OR_EQUIVALENT_TO: Rel = Rel::new('≿', RelCategory::Default);
pub const DOES_NOT_PRECEDE: Rel = Rel::new('⊀', RelCategory::Default);
pub const DOES_NOT_SUCCEED: Rel = Rel::new('⊁', RelCategory::Default);
pub const SUBSET_OF: Rel = Rel::new('⊂', RelCategory::Default);
pub const SUPERSET_OF: Rel = Rel::new('⊃', RelCategory::Default);
pub const NOT_A_SUBSET_OF: Rel = Rel::new('⊄', RelCategory::Default);
pub const NOT_A_SUPERSET_OF: Rel = Rel::new('⊅', RelCategory::Default);
pub const SUBSET_OF_OR_EQUAL_TO: Rel = Rel::new('⊆', RelCategory::Default);
pub const SUPERSET_OF_OR_EQUAL_TO: Rel = Rel::new('⊇', RelCategory::Default);
pub const NEITHER_A_SUBSET_OF_NOR_EQUAL_TO: Rel = Rel::new('⊈', RelCategory::Default);
pub const NEITHER_A_SUPERSET_OF_NOR_EQUAL_TO: Rel = Rel::new('⊉', RelCategory::Default);
pub const SUBSET_OF_WITH_NOT_EQUAL_TO: Rel = Rel::new('⊊', RelCategory::Default);
pub const SUPERSET_OF_WITH_NOT_EQUAL_TO: Rel = Rel::new('⊋', RelCategory::Default);
// pub const MULTISET: Bin = Bin::new('⊌', BinCategory::BD);
// pub const MULTISET_MULTIPLICATION: Bin = Bin::new('⊍', BinCategory::BD);
pub const MULTISET_UNION: Bin = Bin::new('⊎', BinCategory::BD);
pub const SQUARE_IMAGE_OF: Rel = Rel::new('⊏', RelCategory::Default);
pub const SQUARE_ORIGINAL_OF: Rel = Rel::new('⊐', RelCategory::Default);
pub const SQUARE_IMAGE_OF_OR_EQUAL_TO: Rel = Rel::new('⊑', RelCategory::Default);
pub const SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Rel = Rel::new('⊒', RelCategory::Default);
pub const SQUARE_CAP: Bin = Bin::new('⊓', BinCategory::BD);
pub const SQUARE_CUP: Bin = Bin::new('⊔', BinCategory::BD);
pub const CIRCLED_PLUS: Bin = Bin::new('⊕', BinCategory::BD);
pub const CIRCLED_MINUS: Bin = Bin::new('⊖', BinCategory::BD);
pub const CIRCLED_TIMES: Op = Op::new('⊗', OpCategory::C);
pub const CIRCLED_DIVISION_SLASH: Bin = Bin::new('⊘', BinCategory::BD);
pub const CIRCLED_DOT_OPERATOR: Op = Op::new('⊙', OpCategory::C);
pub const CIRCLED_RING_OPERATOR: Op = Op::new('⊚', OpCategory::C);
pub const CIRCLED_ASTERISK_OPERATOR: Op = Op::new('⊛', OpCategory::C);
// pub const CIRCLED_EQUALS: Op = Op('⊜');
pub const CIRCLED_DASH: Bin = Bin::new('⊝', BinCategory::BD);
pub const SQUARED_PLUS: Bin = Bin::new('⊞', BinCategory::BD);
pub const SQUARED_MINUS: Bin = Bin::new('⊟', BinCategory::BD);
pub const SQUARED_TIMES: Op = Op::new('⊠', OpCategory::C);
pub const SQUARED_DOT_OPERATOR: Op = Op::new('⊡', OpCategory::C);
pub const RIGHT_TACK: Rel = Rel::new('⊢', RelCategory::Default);
pub const LEFT_TACK: Rel = Rel::new('⊣', RelCategory::Default);
pub const DOWN_TACK: char = '⊤';
pub const UP_TACK: char = '⊥';
// pub const ASSERTION: Op = Op('⊦');
// pub const MODELS: Op = Op('⊧');
pub const TRUE: Rel = Rel::new('⊨', RelCategory::Default);
pub const FORCES: Rel = Rel::new('⊩', RelCategory::Default);
pub const TRIPLE_VERTICAL_BAR_RIGHT_TURNSTILE: Rel = Rel::new('⊪', RelCategory::Default);
pub const DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Rel = Rel::new('⊫', RelCategory::Default);
pub const DOES_NOT_PROVE: Rel = Rel::new('⊬', RelCategory::Default);
pub const NOT_TRUE: Rel = Rel::new('⊭', RelCategory::Default);
pub const DOES_NOT_FORCE: Rel = Rel::new('⊮', RelCategory::Default);
pub const NEGATED_DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Rel =
    Rel::new('⊯', RelCategory::Default);
// pub const PRECEDES_UNDER_RELATION: Op = Op('⊰');
// pub const SUCCEEDS_UNDER_RELATION: Op = Op('⊱');
pub const NORMAL_SUBGROUP_OF: Rel = Rel::new('⊲', RelCategory::Default);
pub const CONTAINS_AS_NORMAL_SUBGROUP: Rel = Rel::new('⊳', RelCategory::Default);
pub const NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Rel = Rel::new('⊴', RelCategory::Default);
pub const CONTAINS_AS_NORMAL_SUBGROUP_OR_EQUAL_TO: Rel = Rel::new('⊵', RelCategory::Default);
// pub const ORIGINAL_OF: Op = Op('⊶');
// pub const IMAGE_OF: Op = Op('⊷');
pub const MULTIMAP: Rel = Rel::new('⊸', RelCategory::Default);
// pub const HERMITIAN_CONJUGATE_MATRIX: Op = Op('⊹');
pub const INTERCALATE: Rel = Rel::new('⊺', RelCategory::Default);
pub const XOR: Bin = Bin::new('⊻', BinCategory::BD);
pub const NAND: Bin = Bin::new('⊼', BinCategory::BD);
// pub const NOR: Op = Op('⊽');
// pub const RIGHT_ANGLE_WITH_ARC: Op = Op('⊾');
// pub const RIGHT_TRIANGLE: Op = Op('⊿');
pub const N_ARY_LOGICAL_AND: Op = Op::new('⋀', OpCategory::J);
pub const N_ARY_LOGICAL_OR: Op = Op::new('⋁', OpCategory::J);
pub const N_ARY_INTERSECTION: Op = Op::new('⋂', OpCategory::J);
pub const N_ARY_UNION: Op = Op::new('⋃', OpCategory::J);
pub const DIAMOND_OPERATOR: Op = Op::new('⋄', OpCategory::C);
// pub const DOT_OPERATOR: Bin = Bin::new('⋅', BinCategory::BD);
pub const STAR_OPERATOR: Op = Op::new('⋆', OpCategory::C);
pub const DIVISION_TIMES: Op = Op::new('⋇', OpCategory::C);
pub const BOWTIE: Rel = Rel::new('⋈', RelCategory::Default);
pub const LEFT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op::new('⋉', OpCategory::C);
pub const RIGHT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op::new('⋊', OpCategory::C);
pub const LEFT_SEMIDIRECT_PRODUCT: Op = Op::new('⋋', OpCategory::C);
pub const RIGHT_SEMIDIRECT_PRODUCT: Op = Op::new('⋌', OpCategory::C);
pub const REVERSED_TILDE_EQUALS: Rel = Rel::new('⋍', RelCategory::Default);
pub const CURLY_LOGICAL_OR: Bin = Bin::new('⋎', BinCategory::BD);
pub const CURLY_LOGICAL_AND: Bin = Bin::new('⋏', BinCategory::BD);
pub const DOUBLE_SUBSET: Rel = Rel::new('⋐', RelCategory::Default);
pub const DOUBLE_SUPERSET: Rel = Rel::new('⋑', RelCategory::Default);
pub const DOUBLE_INTERSECTION: Bin = Bin::new('⋒', BinCategory::BD);
pub const DOUBLE_UNION: Bin = Bin::new('⋓', BinCategory::BD);
pub const PITCHFORK: Rel = Rel::new('⋔', RelCategory::Default);
// pub const EQUAL_AND_PARALLEL_TO: Op = Op('⋕');
pub const LESS_THAN_WITH_DOT: Rel = Rel::new('⋖', RelCategory::Default);
// pub const GREATER_THAN_WITH_DOT: Rel = Rel::new('⋗', RelCategory::Default);
pub const VERY_MUCH_LESS_THAN: Rel = Rel::new('⋘', RelCategory::Default);
// pub const VERY_MUCH_GREATER_THAN: Rel = Rel::new('⋙', RelCategory::Default);
pub const LESS_THAN_EQUAL_TO_OR_GREATER_THAN: Rel = Rel::new('⋚', RelCategory::Default);
pub const GREATER_THAN_EQUAL_TO_OR_LESS_THAN: Rel = Rel::new('⋛', RelCategory::Default);
// pub const EQUAL_TO_OR_LESS_THAN: Rel = Rel::new('⋜', RelCategory::Default);
// pub const EQUAL_TO_OR_GREATER_THAN: Rel = Rel::new('⋝', RelCategory::Default);
pub const EQUAL_TO_OR_PRECEDES: Rel = Rel::new('⋞', RelCategory::Default);
pub const EQUAL_TO_OR_SUCCEEDS: Rel = Rel::new('⋟', RelCategory::Default);
pub const DOES_NOT_PRECEDE_OR_EQUAL: Rel = Rel::new('⋠', RelCategory::Default);
pub const DOES_NOT_SUCCEED_OR_EQUAL: Rel = Rel::new('⋡', RelCategory::Default);
// pub const NOT_SQUARE_IMAGE_OF_OR_EQUAL_TO: Rel = Rel::new('⋢', RelCategory::Default);
// pub const NOT_SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Rel = Rel::new('⋣', RelCategory::Default);
// pub const SQUARE_IMAGE_OF_OR_NOT_EQUAL_TO: Rel = Rel::new('⋤', RelCategory::Default);
// pub const SQUARE_ORIGINAL_OF_OR_NOT_EQUAL_TO: Rel = Rel::new('⋥', RelCategory::Default);
pub const LESS_THAN_BUT_NOT_EQUIVALENT_TO: Rel = Rel::new('⋦', RelCategory::Default);
pub const GREATER_THAN_BUT_NOT_EQUIVALENT_TO: Rel = Rel::new('⋧', RelCategory::Default);
pub const PRECEDES_BUT_NOT_EQUIVALENT_TO: Rel = Rel::new('⋨', RelCategory::Default);
pub const SUCCEEDS_BUT_NOT_EQUIVALENT_TO: Rel = Rel::new('⋩', RelCategory::Default);
// pub const NOT_NORMAL_SUBGROUP_OF: Rel = Rel::new('⋪', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP: Rel = Rel::new('⋫', RelCategory::Default);
// pub const NOT_NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Rel = Rel::new('⋬', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP_OR_EQUAL: Rel = Rel::new('⋭', RelCategory::Default);
pub const VERTICAL_ELLIPSIS: Rel = Rel::new('⋮', RelCategory::Default);
// pub const MIDLINE_HORIZONTAL_ELLIPSIS: Rel = Rel::new('⋯', RelCategory::Default);
// pub const UP_RIGHT_DIAGONAL_ELLIPSIS: Op = Op('⋰');
pub const DOWN_RIGHT_DIAGONAL_ELLIPSIS: Rel = Rel::new('⋱', RelCategory::Default);
// pub const ELEMENT_OF_WITH_LONG_HORIZONTAL_STROKE: Op = Op('⋲');
// pub const ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋳');
// pub const SMALL_ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋴');
// pub const ELEMENT_OF_WITH_DOT_ABOVE: Op = Op('⋵');
// pub const ELEMENT_OF_WITH_OVERBAR: Op = Op('⋶');
// pub const SMALL_ELEMENT_OF_WITH_OVERBAR: Op = Op('⋷');
// pub const ELEMENT_OF_WITH_UNDERBAR: Op = Op('⋸');
// pub const ELEMENT_OF_WITH_TWO_HORIZONTAL_STROKES: Op = Op('⋹');
// pub const CONTAINS_WITH_LONG_HORIZONTAL_STROKE: Op = Op('⋺');
// pub const CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋻');
// pub const SMALL_CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋼');
// pub const CONTAINS_WITH_OVERBAR: Op = Op('⋽');
// pub const SMALL_CONTAINS_WITH_OVERBAR: Op = Op('⋾');
// pub const Z_NOTATION_BAG_MEMBERSHIP: Op = Op('⋿');

//
// Unicode Block: Miscellaneous Technical
//
pub const LEFT_CEILING: OrdLike = OrdLike::new('⌈', OrdCategory::F);
pub const RIGHT_CEILING: OrdLike = OrdLike::new('⌉', OrdCategory::G);
pub const LEFT_FLOOR: OrdLike = OrdLike::new('⌊', OrdCategory::F);
pub const RIGHT_FLOOR: OrdLike = OrdLike::new('⌋', OrdCategory::G);
pub const TOP_LEFT_CORNER: char = '⌜';
pub const TOP_RIGHT_CORNER: char = '⌝';
pub const BOTTOM_LEFT_CORNER: char = '⌞';
pub const BOTTOM_RIGHT_CORNER: char = '⌟';
pub const FROWN: Rel = Rel::new('⌢', RelCategory::Default);
pub const SMILE: Rel = Rel::new('⌣', RelCategory::Default);
pub const TOP_SQUARE_BRACKET: OrdLike = OrdLike::new('⎴', OrdCategory::I);
pub const BOTTOM_SQUARE_BRACKET: OrdLike = OrdLike::new('⎵', OrdCategory::I);
pub const TOP_PARENTHESIS: OrdLike = OrdLike::new('⏜', OrdCategory::I);
pub const BOTTOM_PARENTHESIS: OrdLike = OrdLike::new('⏝', OrdCategory::I);
pub const TOP_CURLY_BRACKET: OrdLike = OrdLike::new('⏞', OrdCategory::I);
pub const BOTTOM_CURLY_BRACKET: OrdLike = OrdLike::new('⏟', OrdCategory::I);

//
// Unicode Block: Enclosed Alphanumerics
//
pub const CIRCLED_LATIN_CAPITAL_LETTER_R: char = 'Ⓡ'; // not treated as operator
pub const CIRCLED_LATIN_CAPITAL_LETTER_S: char = 'Ⓢ'; // not treated as operator

//
// Unicode Block: Geometric Shapes
//
pub const BLACK_SQUARE: char = '■';
pub const WHITE_SQUARE: char = '□';

pub const BLACK_UP_POINTING_TRIANGLE: char = '▲';
pub const WHITE_UP_POINTING_TRIANGLE: char = '△';

pub const BLACK_RIGHT_POINTING_TRIANGLE: char = '▶';
pub const WHITE_RIGHT_POINTING_TRIANGLE: char = '▷';

pub const BLACK_DOWN_POINTING_TRIANGLE: char = '▼';
pub const WHITE_DOWN_POINTING_TRIANGLE: char = '▽';

pub const BLACK_LEFT_POINTING_TRIANGLE: char = '◀';
pub const WHITE_LEFT_POINTING_TRIANGLE: char = '◁';

pub const LOZENGE: char = '◊';

pub const LARGE_CIRCLE: char = '◯';

pub const WHITE_MEDIUM_SQUARE: char = '◻';

//
// Unicode Block: Miscellaneous Symbols
//
pub const BLACK_STAR: char = '★';

pub const SUN: char = '☉';
pub const ASCENDING_NODE: char = '☊';

pub const WHITE_SUN_WITH_RAYS: char = '☼';

pub const MERCURY: char = '☿';
pub const FEMALE_SIGN: char = '♀';
pub const EARTH: char = '♁';
pub const MALE_SIGN: char = '♂';
pub const JUPITER: char = '♃';
pub const SATURN: char = '♄';
pub const URANUS: char = '♅';
pub const NEPTUNE: char = '♆';
// pub const PLUTO: char = '♇';

pub const BLACK_SPADE_SUIT: char = '♠';
pub const WHITE_HEART_SUIT: char = '♡';
pub const WHITE_DIAMOND_SUIT: char = '♢';
pub const BLACK_CLUB_SUIT: char = '♣';
// pub const WHITE_SPADE_SUIT: char = '♤';
// pub const BLACK_HEART_SUIT: char = '♥';
// pub const BLACK_DIAMOND_SUIT: char = '♦';
// pub const WHITE_CLUB_SUIT: char = '♧';

//
// Unicode Block: Dingbats
//
pub const MALTESE_CROSS: char = '✠';

//
// Unicode Block: Miscellaneous Mathematical Symbols-A
//
pub const PERPENDICULAR: Rel = Rel::new('⟂', RelCategory::Default);

pub const MATHEMATICAL_LEFT_WHITE_SQUARE_BRACKET: OrdLike = OrdLike::new('⟦', OrdCategory::F);
pub const MATHEMATICAL_RIGHT_WHITE_SQUARE_BRACKET: OrdLike = OrdLike::new('⟧', OrdCategory::G);
pub const MATHEMATICAL_LEFT_ANGLE_BRACKET: OrdLike = OrdLike::new('⟨', OrdCategory::F);
pub const MATHEMATICAL_RIGHT_ANGLE_BRACKET: OrdLike = OrdLike::new('⟩', OrdCategory::G);
pub const MATHEMATICAL_LEFT_DOUBLE_ANGLE_BRACKET: OrdLike = OrdLike::new('⟪', OrdCategory::F);
pub const MATHEMATICAL_RIGHT_DOUBLE_ANGLE_BRACKET: OrdLike = OrdLike::new('⟫', OrdCategory::G);
// pub const MATHEMATICAL_LEFT_WHITE_TORTOISE_SHELL_BRACKET: OrdLike = OrdLike::new('⟬', OrdCategory::F);
// pub const MATHEMATICAL_RIGHT_WHITE_TORTOISE_SHELL_BRACKET: OrdLike = OrdLike::new('⟭', OrdCategory::G);
pub const MATHEMATICAL_LEFT_FLATTENED_PARENTHESIS: OrdLike = OrdLike::new('⟮', OrdCategory::F);
pub const MATHEMATICAL_RIGHT_FLATTENED_PARENTHESIS: OrdLike = OrdLike::new('⟯', OrdCategory::G);

//
// Unicode Block: Supplemental Arrows-A
//
pub const LONG_LEFTWARDS_ARROW: Rel = Rel::new('⟵', RelCategory::Default);
pub const LONG_RIGHTWARDS_ARROW: Rel = Rel::new('⟶', RelCategory::Default);
pub const LONG_LEFT_RIGHT_ARROW: Rel = Rel::new('⟷', RelCategory::Default);
pub const LONG_LEFTWARDS_DOUBLE_ARROW: Rel = Rel::new('⟸', RelCategory::Default);
pub const LONG_RIGHTWARDS_DOUBLE_ARROW: Rel = Rel::new('⟹', RelCategory::Default);
pub const LONG_LEFT_RIGHT_DOUBLE_ARROW: Rel = Rel::new('⟺', RelCategory::Default);
// pub const LONG_LEFTWARDS_ARROW_FROM_BAR: Op = Op('⟻');
pub const LONG_RIGHTWARDS_ARROW_FROM_BAR: Rel = Rel::new('⟼', RelCategory::Default);

//
// Unicode Block: Supplemental Arrows-B
//
pub const LEFTWARDS_ARROW_TAIL: Rel = Rel::new('⤙', RelCategory::Default);
pub const RIGHTWARDS_ARROW_TAIL: Rel = Rel::new('⤚', RelCategory::Default);

//
// Unicode Block: Miscellaneous Mathematical Symbols-B
//
pub const LEFT_WHITE_CURLY_BRACKET: OrdLike = OrdLike::new('⦃', OrdCategory::F);
pub const RIGHT_WHITE_CURLY_BRACKET: OrdLike = OrdLike::new('⦄', OrdCategory::G);
// pub const LEFT_WHITE_PARENTHESIS: Fence = fence('⦅', Stretchy::Always);
// pub const RIGHT_WHITE_PARENTHESIS: Fence = fence('⦆', false, Stretchy::Always);
pub const Z_NOTATION_LEFT_IMAGE_BRACKET: OrdLike = OrdLike::new('⦇', OrdCategory::F);
pub const Z_NOTATION_RIGHT_IMAGE_BRACKET: OrdLike = OrdLike::new('⦈', OrdCategory::G);
pub const Z_NOTATION_LEFT_BINDING_BRACKET: OrdLike = OrdLike::new('⦉', OrdCategory::F);
pub const Z_NOTATION_RIGHT_BINDING_BRACKET: OrdLike = OrdLike::new('⦊', OrdCategory::G);

pub const SQUARED_RISING_DIAGONAL_SLASH: Bin = Bin::new('⧄', BinCategory::BD);
pub const SQUARED_FALLING_DIAGONAL_SLASH: Bin = Bin::new('⧅', BinCategory::BD);
pub const SQUARED_SQUARE: Bin = Bin::new('⧈', BinCategory::BD);
pub const BLACK_LOZENGE: char = '⧫';

// pub const REVERSE_SOLIDUS_OPERATOR: Bin = Bin::new('⧵', BinCategory::BD);

//
// Unicode Block: Supplemental Mathematical Operators
//
pub const N_ARY_CIRCLED_DOT_OPERATOR: Op = Op::new('⨀', OpCategory::J);
pub const N_ARY_CIRCLED_PLUS_OPERATOR: Op = Op::new('⨁', OpCategory::J);
pub const N_ARY_CIRCLED_TIMES_OPERATOR: Op = Op::new('⨂', OpCategory::J);
pub const N_ARY_UNION_OPERATOR_WITH_DOT: Op = Op::new('⨃', OpCategory::J);
pub const N_ARY_UNION_OPERATOR_WITH_PLUS: Op = Op::new('⨄', OpCategory::J);
pub const N_ARY_SQUARE_INTERSECTION_OPERATOR: Op = Op::new('⨅', OpCategory::J);
pub const N_ARY_SQUARE_UNION_OPERATOR: Op = Op::new('⨆', OpCategory::J);
pub const TWO_LOGICAL_AND_OPERATOR: Op = Op::new('⨇', OpCategory::J);
pub const TWO_LOGICAL_OR_OPERATOR: Op = Op::new('⨈', OpCategory::J);
pub const N_ARY_TIMES_OPERATOR: Op = Op::new('⨉', OpCategory::J);
// pub const MODULO_TWO_SUM: Op = Op('⨊');
pub const SUMMATION_WITH_INTEGRAL: Op = Op::new('⨋', OpCategory::H);
pub const QUADRUPLE_INTEGRAL_OPERATOR: Op = Op::new('⨌', OpCategory::H);
pub const FINITE_PARTL_INTEGRAL: Op = Op::new('⨍', OpCategory::H);
pub const INTEGRAL_WITH_DOUBLE_STROKE: Op = Op::new('⨎', OpCategory::H);
pub const INTEGRAL_AVERAGE_WITH_SLASH: Op = Op::new('⨏', OpCategory::H);
pub const CIRCULATION_FUNCTION: Op = Op::new('⨐', OpCategory::H);
pub const ANTICLOCKWISE_INTEGRATION: Op = Op::new('⨑', OpCategory::H);
// pub const LINE_INTEGRATION_WITH_RECTANGULAR_PATH_AROUND_POLE: Op = Op('⨒');
// pub const LINE_INTEGRATION_WITH_SEMICIRCULAR_PATH_AROUND_POLE: Op = Op('⨓');
// pub const LINE_INTEGRATION_NOT_INCLUDING_THE_POLE: Op = Op('⨔');
// pub const INTEGRAL_AROUND_A_POINT_OPERATOR: Op = Op('⨕');
// pub const QUATERNION_INTEGRAL_OPERATOR: Op = Op('⨖');
// pub const INTEGRAL_WITH_LEFTWARDS_ARROW_WITH_HOOK: Op = Op('⨗');
// pub const INTEGRAL_WITH_TIMES_SIGN: Op = Op('⨘');
// pub const INTEGRAL_WITH_INTERSECTION: Op = Op('⨙');
// pub const INTEGRAL_WITH_UNION: Op = Op('⨚');
// pub const INTEGRAL_WITH_OVERBAR: Op = Op('⨛');
// pub const INTEGRAL_WITH_UNDERBAR: Op = Op('⨜');
// pub const JOIN: Op = Op('⨝');
// pub const LARGE_LEFT_TRIANGLE_OPERATOR: Op = Op('⨞');
pub const Z_NOTATION_SCHEMA_COMPOSITION: Rel = Rel::new('⨟', RelCategory::Default);
// pub const Z_NOTATION_SCHEMA_PIPING: Op = Op('⨠');
// pub const Z_NOTATION_SCHEMA_PROJECTION: Op = Op('⨡');
// pub const PLUS_SIGN_WITH_SMALL_CIRCLE_ABOVE: Bin = Bin::new('⨢', BinCategory::BD);
// pub const PLUS_SIGN_WITH_CIRCUMFLEX_ACCENT_ABOVE: Bin = Bin::new('⨣', BinCategory::BD);
// pub const PLUS_SIGN_WITH_TILDE_ABOVE: Bin = Bin::new('⨤', BinCategory::BD);
// pub const PLUS_SIGN_WITH_DOT_BELOW: Bin = Bin::new('⨥', BinCategory::BD);
// pub const PLUS_SIGN_WITH_TILDE_BELOW: Bin = Bin::new('⨦', BinCategory::BD);
// pub const PLUS_SIGN_WITH_SUBSCRIPT_TWO: Bin = Bin::new('⨧', BinCategory::BD);
// pub const PLUS_SIGN_WITH_BLACK_TRIANGLE: Bin = Bin::new('⨨', BinCategory::BD);
// pub const MINUS_SIGN_WITH_COMMA_ABOVE: Bin = Bin::new('⨩', BinCategory::BD);
// pub const MINUS_SIGN_WITH_DOT_BELOW: Bin = Bin::new('⨪', BinCategory::BD);
// pub const MINUS_SIGN_WITH_FALLING_DOTS: Bin = Bin::new('⨫', BinCategory::BD);
// pub const MINUS_SIGN_WITH_RISING_DOTS: Bin = Bin::new('⨬', BinCategory::BD);
// pub const PLUS_SIGN_IN_LEFT_HALF_CIRCLE: Bin = Bin::new('⨭', BinCategory::BD);
// pub const PLUS_SIGN_IN_RIGHT_HALF_CIRCLE: Bin = Bin::new('⨮', BinCategory::BD);
// pub const VECTOR_OR_CROSS_PRODUCT: Bin = Bin::new('⨯', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_WITH_DOT_ABOVE: Bin = Bin::new('⨰', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_WITH_UNDERBAR: Bin = Bin::new('⨱', BinCategory::BD);
// pub const SEMIDIRECT_PRODUCT_WITH_BOTTOM_CLOSED: Bin = Bin::new('⨲', BinCategory::BD);
// pub const SMASH_PRODUCT: Bin = Bin::new('⨳', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_IN_LEFT_HALF_CIRCLE: Bin = Bin::new('⨴', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_IN_RIGHT_HALF_CIRCLE: Bin = Bin::new('⨵', BinCategory::BD);
// pub const CIRCLED_MULTIPLICATION_SIGN_WITH_CIRCUMFLEX_ACCENT: Op = Op('⨶');
// pub const MULTIPLICATION_SIGN_IN_DOUBLE_CIRCLE: Op = Op('⨷');
// pub const CIRCLED_DIVISION_SIGN: Op = Op('⨸');
// pub const PLUS_SIGN_IN_TRIANGLE: Op = Op('⨹');
// pub const MINUS_SIGN_IN_TRIANGLE: Op = Op('⨺');
// pub const MULTIPLICATION_SIGN_IN_TRIANGLE: Op = Op('⨻');
// pub const INTERIOR_PRODUCT: Op = Op('⨼');
// pub const RIGHTHAND_INTERIOR_PRODUCT: Op = Op('⨽');
// pub const Z_NOTATION_RELATIONAL_COMPOSITION: Op = Op('⨾');
pub const AMALGAMATION_OR_COPRODUCT: Rel = Rel::new('⨿', RelCategory::Default);
// pub const INTERSECTION_WITH_DOT: Op = Op('⩀');
// pub const UNION_WITH_MINUS_SIGN: Op = Op('⩁');
// pub const UNION_WITH_OVERBAR: Op = Op('⩂');
// pub const INTERSECTION_WITH_OVERBAR: Op = Op('⩃');
// pub const INTERSECTION_WITH_LOGICAL_AND: Op = Op('⩄');
// pub const UNION_WITH_LOGICAL_OR: Op = Op('⩅');
// pub const UNION_ABOVE_INTERSECTION: Op = Op('⩆');
// pub const INTERSECTION_ABOVE_UNION: Op = Op('⩇');
// pub const UNION_ABOVE_BAR_ABOVE_INTERSECTION: Op = Op('⩈');
// pub const INTERSECTION_ABOVE_BAR_ABOVE_UNION: Op = Op('⩉');
// pub const UNION_BESIDE_AND_JOINED_WITH_UNION: Op = Op('⩊');
// pub const INTERSECTION_BESIDE_AND_JOINED_WITH_INTERSECTION: Op = Op('⩋');
// pub const CLOSED_UNION_WITH_SERIFS: Op = Op('⩌');
// pub const CLOSED_INTERSECTION_WITH_SERIFS: Op = Op('⩍');
// pub const DOUBLE_SQUARE_INTERSECTION: Op = Op('⩎');
// pub const DOUBLE_SQUARE_UNION: Op = Op('⩏');
// pub const CLOSED_UNION_WITH_SERIFS_AND_SMASH_PRODUCT: Op = Op('⩐');
// pub const LOGICAL_AND_WITH_DOT_ABOVE: Op = Op('⩑');
// pub const LOGICAL_OR_WITH_DOT_ABOVE: Op = Op('⩒');
// pub const DOUBLE_LOGICAL_AND: Op = Op('⩓');
// pub const DOUBLE_LOGICAL_OR: Op = Op('⩔');
// pub const TWO_INTERSECTING_LOGICAL_AND: Op = Op('⩕');
// pub const TWO_INTERSECTING_LOGICAL_OR: Op = Op('⩖');
// pub const SLOPING_LARGE_OR: Op = Op('⩗');
// pub const SLOPING_LARGE_AND: Op = Op('⩘');
// pub const LOGICAL_OR_OVERLAPPING_LOGICAL_AND: Op = Op('⩙');
// pub const LOGICAL_AND_WITH_MIDDLE_STEM: Op = Op('⩚');
// pub const LOGICAL_OR_WITH_MIDDLE_STEM: Op = Op('⩛');
// pub const LOGICAL_AND_WITH_HORIZONTAL_DASH: Op = Op('⩜');
// pub const LOGICAL_OR_WITH_HORIZONTAL_DASH: Op = Op('⩝');
pub const LOGICAL_AND_WITH_DOUBLE_OVERBAR: Rel = Rel::new('⩞', RelCategory::Default);
// pub const LOGICAL_AND_WITH_UNDERBAR: Op = Op('⩟');
// pub const LOGICAL_AND_WITH_DOUBLE_UNDERBAR: Op = Op('⩠');
// pub const SMALL_VEE_WITH_UNDERBAR: Op = Op('⩡');
// pub const LOGICAL_OR_WITH_DOUBLE_OVERBAR: Op = Op('⩢');
// pub const LOGICAL_OR_WITH_DOUBLE_UNDERBAR: Op = Op('⩣');
// pub const Z_NOTATION_DOMAIN_ANTIRESTRICTION: Op = Op('⩤');
// pub const Z_NOTATION_RANGE_ANTIRESTRICTION: Op = Op('⩥');
pub const EQUALS_SIGN_WITH_DOT_BELOW: Rel = Rel::new('⩦', RelCategory::Default);
// pub const IDENTICAL_WITH_DOT_ABOVE: Op = Op('⩧');
// pub const TRIPLE_HORIZONTAL_BAR_WITH_DOUBLE_VERTICAL_STROKE: Op = Op('⩨');
// pub const TRIPLE_HORIZONTAL_BAR_WITH_TRIPLE_VERTICAL_STROKE: Op = Op('⩩');
// pub const TILDE_OPERATOR_WITH_DOT_ABOVE: Op = Op('⩪');
// pub const TILDE_OPERATOR_WITH_RISING_DOTS: Op = Op('⩫');
// pub const SIMILAR_MINUS_SIMILAR: Op = Op('⩬');
// pub const CONGRUENT_WITH_DOT_ABOVE: Op = Op('⩭');
// pub const EQUALS_WITH_ASTERISK: Op = Op('⩮');
// pub const ALMOST_EQUAL_TO_WITH_CIRCUMFLEX_ACCENT: Op = Op('⩯');
// pub const APPROXIMATELY_EQUAL_OR_EQUAL_TO: Op = Op('⩰');
// pub const EQUALS_SIGN_ABOVE_PLUS_SIGN: Op = Op('⩱');
// pub const PLUS_SIGN_ABOVE_EQUALS_SIGN: Op = Op('⩲');
// pub const EQUALS_SIGN_ABOVE_TILDE_OPERATOR: Op = Op('⩳');
// pub const DOUBLE_COLON_EQUAL: Op = Op('⩴');
// pub const TWO_CONSECUTIVE_EQUALS_SIGNS: Op = Op('⩵');
// pub const THREE_CONSECUTIVE_EQUALS_SIGNS: Op = Op('⩶');
// pub const EQUALS_SIGN_WITH_TWO_DOTS_ABOVE_AND_TWO_DOTS_BELOW: Op = Op('⩷');
// pub const EQUIVALENT_WITH_FOUR_DOTS_ABOVE: Op = Op('⩸');
// pub const LESS_THAN_WITH_CIRCLE_INSIDE: Op = Op('⩹');
// pub const GREATER_THAN_WITH_CIRCLE_INSIDE: Op = Op('⩺');
// pub const LESS_THAN_WITH_QUESTION_MARK_ABOVE: Op = Op('⩻');
// pub const GREATER_THAN_WITH_QUESTION_MARK_ABOVE: Op = Op('⩼');
pub const LESS_THAN_OR_SLANTED_EQUAL_TO: Rel = Rel::new('⩽', RelCategory::Default);
pub const GREATER_THAN_OR_SLANTED_EQUAL_TO: Rel = Rel::new('⩾', RelCategory::Default);
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⩿');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⪀');
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪁');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪂');
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_RIGHT: Op = Op('⪃');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_LEFT: Op = Op('⪄');
pub const LESS_THAN_OR_APPROXIMATE: Rel = Rel::new('⪅', RelCategory::Default);
pub const GREATER_THAN_OR_APPROXIMATE: Rel = Rel::new('⪆', RelCategory::Default);
pub const LESS_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Rel = Rel::new('⪇', RelCategory::Default);
pub const GREATER_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Rel = Rel::new('⪈', RelCategory::Default);
pub const LESS_THAN_AND_NOT_APPROXIMATE: Rel = Rel::new('⪉', RelCategory::Default);
pub const GREATER_THAN_AND_NOT_APPROXIMATE: Rel = Rel::new('⪊', RelCategory::Default);
pub const LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_GREATER_THAN: Rel =
    Rel::new('⪋', RelCategory::Default);
pub const GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_LESS_THAN: Rel =
    Rel::new('⪌', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SIMILAR_OR_EQUAL: Rel = Rel::new('⪍', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SIMILAR_OR_EQUAL: Rel = Rel::new('⪎', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SIMILAR_ABOVE_GREATER_THAN: Rel = Rel::new('⪏', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SIMILAR_ABOVE_LESS_THAN: Rel = Rel::new('⪐', RelCategory::Default);
// pub const LESS_THAN_ABOVE_GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL: Rel = Rel::new('⪑', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL: Rel = Rel::new('⪒', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SLANTED_EQUAL_ABOVE_GREATER_THAN_ABOVE_SLANTED_EQUAL: Rel = Rel::new('⪓', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SLANTED_EQUAL_ABOVE_LESS_THAN_ABOVE_SLANTED_EQUAL: Rel = Rel::new('⪔', RelCategory::Default);
pub const SLANTED_EQUAL_TO_OR_LESS_THAN: Rel = Rel::new('⪕', RelCategory::Default);
pub const SLANTED_EQUAL_TO_OR_GREATER_THAN: Rel = Rel::new('⪖', RelCategory::Default);
// pub const SLANTED_EQUAL_TO_OR_LESS_THAN_WITH_DOT_INSIDE: Rel = Rel::new('⪗', RelCategory::Default);
// pub const SLANTED_EQUAL_TO_OR_GREATER_THAN_WITH_DOT_INSIDE: Rel = Rel::new('⪘', RelCategory::Default);
// pub const DOUBLE_LINE_EQUAL_TO_OR_LESS_THAN: Rel = Rel::new('⪙', RelCategory::Default);
// pub const DOUBLE_LINE_EQUAL_TO_OR_GREATER_THAN: Rel = Rel::new('⪚', RelCategory::Default);
// pub const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_LESS_THAN: Rel = Rel::new('⪛', RelCategory::Default);
// pub const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_GREATER_THAN: Rel = Rel::new('⪜', RelCategory::Default);
// pub const SIMILAR_OR_LESS_THAN: Rel = Rel::new('⪝', RelCategory::Default);
// pub const SIMILAR_OR_GREATER_THAN: Rel = Rel::new('⪞', RelCategory::Default);
// pub const SIMILAR_ABOVE_LESS_THAN_ABOVE_EQUALS_SIGN: Rel = Rel::new('⪟', RelCategory::Default);
// pub const SIMILAR_ABOVE_GREATER_THAN_ABOVE_EQUALS_SIGN: Rel = Rel::new('⪠', RelCategory::Default);
// pub const DOUBLE_NESTED_LESS_THAN: Rel = Rel::new('⪡', RelCategory::Default);
// pub const DOUBLE_NESTED_GREATER_THAN: Rel = Rel::new('⪢', RelCategory::Default);
// pub const DOUBLE_NESTED_LESS_THAN_WITH_UNDERBAR: Rel = Rel::new('⪣', RelCategory::Default);
// pub const GREATER_THAN_OVERLAPPING_LESS_THAN: Rel = Rel::new('⪤', RelCategory::Default);
// pub const GREATER_THAN_BESIDE_LESS_THAN: Rel = Rel::new('⪥', RelCategory::Default);
// pub const LESS_THAN_CLOSED_BY_CURVE: Rel = Rel::new('⪦', RelCategory::Default);
// pub const GREATER_THAN_CLOSED_BY_CURVE: Rel = Rel::new('⪧', RelCategory::Default);
// pub const LESS_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Rel = Rel::new('⪨', RelCategory::Default);
// pub const GREATER_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Rel = Rel::new('⪩', RelCategory::Default);
// pub const SMALLER_THAN: Rel = Rel::new('⪪', RelCategory::Default);
// pub const LARGER_THAN: Rel = Rel::new('⪫', RelCategory::Default);
// pub const SMALLER_THAN_OR_EQUAL_TO: Rel = Rel::new('⪬', RelCategory::Default);
// pub const LARGER_THAN_OR_EQUAL_TO: Rel = Rel::new('⪭', RelCategory::Default);
// pub const EQUALS_SIGN_WITH_BUMPY_ABOVE: Rel = Rel::new('⪮', RelCategory::Default);
pub const PRECEDES_ABOVE_SINGLE_LINE_EQUALS_SIGN: Rel = Rel::new('⪯', RelCategory::Default);
pub const SUCCEEDS_ABOVE_SINGLE_LINE_EQUALS_SIGN: Rel = Rel::new('⪰', RelCategory::Default);
// pub const PRECEDES_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Rel = Rel::new('⪱', RelCategory::Default);
// pub const SUCCEEDS_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Rel = Rel::new('⪲', RelCategory::Default);
// pub const PRECEDES_ABOVE_EQUALS_SIGN: Rel = Rel::new('⪳', RelCategory::Default);
// pub const SUCCEEDS_ABOVE_EQUALS_SIGN: Rel = Rel::new('⪴', RelCategory::Default);
pub const PRECEDES_ABOVE_NOT_EQUAL_TO: Rel = Rel::new('⪵', RelCategory::Default);
pub const SUCCEEDS_ABOVE_NOT_EQUAL_TO: Rel = Rel::new('⪶', RelCategory::Default);
pub const PRECEDES_ABOVE_ALMOST_EQUAL_TO: Rel = Rel::new('⪷', RelCategory::Default);
pub const SUCCEEDS_ABOVE_ALMOST_EQUAL_TO: Rel = Rel::new('⪸', RelCategory::Default);
pub const PRECEDES_ABOVE_NOT_ALMOST_EQUAL_TO: Rel = Rel::new('⪹', RelCategory::Default);
pub const SUCCEEDS_ABOVE_NOT_ALMOST_EQUAL_TO: Rel = Rel::new('⪺', RelCategory::Default);
// pub const DOUBLE_PRECEDES: Rel = Rel::new('⪻', RelCategory::Default);
// pub const DOUBLE_SUCCEEDS: Rel = Rel::new('⪼', RelCategory::Default);
// pub const SUBSET_WITH_DOT: Rel = Rel::new('⪽', RelCategory::Default);
// pub const SUPERSET_WITH_DOT: Rel = Rel::new('⪾', RelCategory::Default);
// pub const SUBSET_WITH_PLUS_SIGN_BELOW: Rel = Rel::new('⪿', RelCategory::Default);
// pub const SUPERSET_WITH_PLUS_SIGN_BELOW: Rel = Rel::new('⫀', RelCategory::Default);
// pub const SUBSET_WITH_MULTIPLICATION_SIGN_BELOW: Rel = Rel::new('⫁', RelCategory::Default);
// pub const SUPERSET_WITH_MULTIPLICATION_SIGN_BELOW: Rel = Rel::new('⫂', RelCategory::Default);
// pub const SUBSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Rel = Rel::new('⫃', RelCategory::Default);
// pub const SUPERSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Rel = Rel::new('⫄', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_EQUALS_SIGN: Rel = Rel::new('⫅', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_EQUALS_SIGN: Rel = Rel::new('⫆', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_TILDE_OPERATOR: Rel = Rel::new('⫇', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_TILDE_OPERATOR: Rel = Rel::new('⫈', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_ALMOST_EQUAL_TO: Rel = Rel::new('⫉', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_ALMOST_EQUAL_TO: Rel = Rel::new('⫊', RelCategory::Default);
pub const SUBSET_OF_ABOVE_NOT_EQUAL_TO: Rel = Rel::new('⫋', RelCategory::Default);
pub const SUPERSET_OF_ABOVE_NOT_EQUAL_TO: Rel = Rel::new('⫌', RelCategory::Default);

//
// Unicode Block: Small Form Variants
//
// pub const SMALL_REVERSE_SOLIDUS: Rel = Rel::new('﹨', RelCategory::Default);
