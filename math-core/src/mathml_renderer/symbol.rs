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

macro_rules! make_character_class {
    ($(#[$meta:meta])* $struct_name:ident, $cat_type:ty, $const_fn_name:ident) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Copy)]
        pub struct $struct_name {
            char: u16,
            pub cat: $cat_type,
        }

        const fn $const_fn_name(ch: char, cat: $cat_type) -> $struct_name {
            assert!(ch as u32 <= u16::MAX as u32);
            $struct_name {
                char: ch as u16,
                cat,
            }
        }

        impl $struct_name {
            #[inline(always)]
            pub const fn as_op(&self) -> MathMLOperator {
                debug_assert!(char::from_u32(self.char as u32).is_some());
                MathMLOperator(unsafe { char::from_u32_unchecked(self.char as u32) })
            }
        }

        // We use a custom `Debug` implementation to render the character as a `char`.
        impl Debug for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($struct_name))
                    .field("char", &self.as_op().0)
                    .field("cat", &self.cat)
                    .finish()
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdCategory {
    /// Category F and G: Prefix or postfix, zero spacing, stretchy, symmetric
    /// (e.g. `|`).
    FG,
    /// Category D: Prefix, zero spacing (e.g. `¬`).
    OnlyD,
    /// Category E: Postfix, zero spacing (e.g. `′`).
    OnlyE,
    /// Category I: Postfix, zero spacing, stretchy
    OnlyI,
    /// Category K: Infix, zero spacing (e.g. `/`).
    OnlyK,
}

make_character_class!(
    /// An operator with zero spacing, categories D, E, F, G, I, K.
    OrdLike, OrdCategory, ord
);

impl OrdLike {
    #[inline(always)]
    pub const fn as_stretchable_op(&self) -> Option<StretchableOp> {
        match self.cat {
            OrdCategory::FG => Some(StretchableOp {
                char: self.char,
                stretchy: Stretchy::PrePostfix,
            }),
            OrdCategory::OnlyK => Some(StretchableOp {
                char: self.char,
                stretchy: Stretchy::Never,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BigOpCategory {
    /// Category H: Prefix, op spacing, symmetric, largeop (e.g. `∫`).
    OnlyH,
    /// Category J: Prefix, op spacing, symmetric, largeop, movablelimits
    /// (e.g. `∑`).
    OnlyJ,
}

make_character_class!(
    /// A prefix operator with operator spacing (categories H, J).
    BigOp, BigOpCategory, big_op
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinCategory {
    /// Category B: Infix, binary op spacing (e.g. `÷`)
    OnlyB,
    /// Category B and D: Infix and prefix, binary op spacing (e.g. `+`)
    BD,
    /// Category C: Infix, op spacing (e.g. `×`).
    OnlyC,
}

make_character_class!(
    /// An infix operator that does *not* have relation spacing or zero spacing
    /// (categories B and C).
    Bin, BinCategory, bin
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelCategory {
    Default,
    /// Category A: Infix, relation spacing, stretchy (e.g. `↑`).
    OnlyA,
}

make_character_class!(
    /// A operator with relation spacing.
    Rel, RelCategory, rel
);

impl Rel {
    #[inline(always)]
    pub const fn as_stretchable_op(&self) -> Option<StretchableOp> {
        match self.cat {
            RelCategory::OnlyA => Some(StretchableOp {
                char: self.char,
                stretchy: Stretchy::AlwaysAsymmetric,
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

/// An operator that has zero spacing and is stretchy and symmetric
/// (category G and F).
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(transparent)]
pub struct Fence(char);

impl Fence {
    #[inline(always)]
    pub const fn as_op(&self) -> StretchableOp {
        assert!(self.0 as u32 <= u16::MAX as u32);
        StretchableOp {
            char: self.0 as u16,
            stretchy: Stretchy::Always,
        }
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
    char: u16,
    pub stretchy: Stretchy,
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

impl StretchableOp {
    /// The parenthesis behaves like a normal identifier
    /// (which is different from an operator with reduced spacing!)
    #[inline]
    pub fn ordinary_spacing(&self) -> bool {
        matches!(self.stretchy, Stretchy::Never | Stretchy::PrePostfix)
    }
}

impl From<StretchableOp> for char {
    #[inline]
    fn from(op: StretchableOp) -> Self {
        debug_assert!(char::from_u32(op.char as u32).is_some());
        unsafe { char::from_u32_unchecked(op.char as u32) }
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
pub const AMPERSAND: OrdLike = ord('&', OrdCategory::OnlyE);
// pub const APOSTROPHE: char = '\'';
pub const LEFT_PARENTHESIS: Fence = Fence('(');
pub const RIGHT_PARENTHESIS: Fence = Fence(')');
// pub const ASTERISK: Op = Op('*');
pub const PLUS_SIGN: Bin = bin('+', BinCategory::BD);
pub const COMMA: Punct = Punct(',');
// pub const FULL_STOP: Bin = bin('.', BinCategory::OnlyC);
pub const SOLIDUS: OrdLike = ord('/', OrdCategory::OnlyK);

pub const COLON: Punct = Punct(':');
pub const SEMICOLON: Punct = Punct(';');
// pub const LESS_THAN_SIGN: Op = Op('<');
pub const EQUALS_SIGN: Rel = rel('=', RelCategory::Default);
// pub const GREATER_THAN_SIGN: Op = Op('>');
// pub const QUESTION_MARK: Op = Op('?');
// pub const COMMERCIAL_AT: char = '@';

pub const LEFT_SQUARE_BRACKET: Fence = Fence('[');
pub const REVERSE_SOLIDUS: OrdLike = ord('\\', OrdCategory::OnlyK);
pub const RIGHT_SQUARE_BRACKET: Fence = Fence(']');
// pub const CIRCUMFLEX_ACCENT: Rel = rel('^', RelCategory::Default);
pub const LOW_LINE: Rel = rel('_', RelCategory::Default);
pub const GRAVE_ACCENT: Rel = rel('`', RelCategory::Default);

pub const LEFT_CURLY_BRACKET: Fence = Fence('{');
pub const VERTICAL_LINE: OrdLike = ord('|', OrdCategory::FG);
pub const RIGHT_CURLY_BRACKET: Fence = Fence('}');
pub const TILDE: Rel = rel('~', RelCategory::Default);

//
// Unicode Block: Latin-1 Supplement
//
pub const SECTION_SIGN: char = '§';
pub const DIAERESIS: Rel = rel('¨', RelCategory::Default);
pub const COPYRIGHT_SIGN: char = '©';

pub const NOT_SIGN: OrdLike = ord('¬', OrdCategory::OnlyD);

pub const MACRON: Rel = rel('¯', RelCategory::Default);

pub const PLUS_MINUS_SIGN: Bin = bin('±', BinCategory::BD);

pub const ACUTE_ACCENT: Rel = rel('´', RelCategory::Default);

pub const PILCROW_SIGN: char = '¶';
pub const MIDDLE_DOT: Bin = bin('·', BinCategory::OnlyC);

pub const MULTIPLICATION_SIGN: Bin = bin('×', BinCategory::OnlyC);

pub const LATIN_SMALL_LETTER_ETH: char = 'ð';

pub const DIVISION_SIGN: Bin = bin('÷', BinCategory::BD);

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
pub const CARON: Rel = rel('ˇ', RelCategory::Default);
pub const BREVE: Rel = rel('˘', RelCategory::Default);
pub const DOT_ABOVE: Rel = rel('˙', RelCategory::Default);

//
// Unicode Block: Combining Diacritical Marks
//
pub const COMBINING_GRAVE_ACCENT: char = '\u{300}';
pub const COMBINING_ACUTE_ACCENT: char = '\u{301}';
pub const COMBINING_CIRCUMFLEX_ACCENT: Rel = rel('\u{302}', RelCategory::Default);
pub const COMBINING_TILDE: Rel = rel('\u{303}', RelCategory::Default);
// pub const COMBINING_MACRON: char = '\u{304}';
pub const COMBINING_OVERLINE: char = '\u{305}';
pub const COMBINING_BREVE: char = '\u{306}';
pub const COMBINING_DOT_ABOVE: char = '\u{307}';
pub const COMBINING_DIAERESIS: char = '\u{308}';
// pub const COMBINING_HOOK_ABOVE: char = '\u{309}';
pub const COMBINING_RING_ABOVE: char = '\u{30A}';
pub const COMBINING_DOUBLE_ACUTE_ACCENT: char = '\u{30B}';
pub const COMBINING_CARON: Rel = rel('\u{30C}', RelCategory::Default);

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
pub const DOUBLE_VERTICAL_LINE: OrdLike = ord('‖', OrdCategory::FG);

pub const DAGGER: char = '†';
pub const DOUBLE_DAGGER: char = '‡';

pub const HORIZONTAL_ELLIPSIS: char = '…';
pub const PRIME: OrdLike = ord('′', OrdCategory::OnlyE);
pub const DOUBLE_PRIME: OrdLike = ord('″', OrdCategory::OnlyE);
pub const TRIPLE_PRIME: OrdLike = ord('‴', OrdCategory::OnlyE);
pub const REVERSED_PRIME: OrdLike = ord('‵', OrdCategory::OnlyE);
pub const REVERSED_DOUBLE_PRIME: OrdLike = ord('‶', OrdCategory::OnlyE);
pub const REVERSED_TRIPLE_PRIME: OrdLike = ord('‷', OrdCategory::OnlyE);
// pub const CARET: Ord = ord('‸');
// pub const SINGLE_LEFT_POINTING_ANGLE_QUOTATION_MARK: Ord = ord('‹');
// pub const SINGLE_RIGHT_POINTING_ANGLE_QUOTATION_MARK: Ord = ord('›');
// pub const REFERENCE_MARK: Ord = ord('※');
// pub const DOUBLE_EXCLAMATION_MARK: Ord = ord('‼');
// pub const INTERROBANG: Ord = ord('‽');
pub const OVERLINE: Rel = rel('‾', RelCategory::Default);

pub const QUADRUPLE_PRIME: OrdLike = ord('⁗', OrdCategory::OnlyE);

//
// Unicode Block: Combining Diacritical Marks for Symbols
//
pub const COMBINING_RIGHT_ARROW_ABOVE: Rel = rel('\u{20D7}', RelCategory::Default);

pub const COMBINING_THREE_DOTS_ABOVE: Rel = rel('\u{20DB}', RelCategory::Default);
pub const COMBINING_FOUR_DOTS_ABOVE: Rel = rel('\u{20DC}', RelCategory::Default);

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
pub const LEFTWARDS_ARROW: Rel = rel('←', RelCategory::Default);
pub const UPWARDS_ARROW: Rel = rel('↑', RelCategory::OnlyA);
pub const RIGHTWARDS_ARROW: Rel = rel('→', RelCategory::Default);
pub const DOWNWARDS_ARROW: Rel = rel('↓', RelCategory::OnlyA);
pub const LEFT_RIGHT_ARROW: Rel = rel('↔', RelCategory::Default);
pub const UP_DOWN_ARROW: Rel = rel('↕', RelCategory::OnlyA);
pub const NORTH_WEST_ARROW: Rel = rel('↖', RelCategory::Default);
pub const NORTH_EAST_ARROW: Rel = rel('↗', RelCategory::Default);
pub const SOUTH_EAST_ARROW: Rel = rel('↘', RelCategory::Default);
pub const SOUTH_WEST_ARROW: Rel = rel('↙', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_STROKE: Rel = rel('↚', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_STROKE: Rel = rel('↛', RelCategory::Default);
// pub const LEFTWARDS_WAVE_ARROW: Rel = rel('↜', RelCategory::Default);
// pub const RIGHTWARDS_WAVE_ARROW: Rel = rel('↝', RelCategory::Default);
pub const LEFTWARDS_TWO_HEADED_ARROW: Rel = rel('↞', RelCategory::Default);
// pub const UPWARDS_TWO_HEADED_ARROW: Rel = rel('↟', RelCategory::Default);
pub const RIGHTWARDS_TWO_HEADED_ARROW: Rel = rel('↠', RelCategory::Default);
// pub const DOWNWARDS_TWO_HEADED_ARROW: Rel = rel('↡', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_TAIL: Rel = rel('↢', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_TAIL: Rel = rel('↣', RelCategory::Default);
// pub const LEFTWARDS_ARROW_FROM_BAR: Rel = rel('↤', RelCategory::Default);
// pub const UPWARDS_ARROW_FROM_BAR: Rel = rel('↥', RelCategory::Default);
pub const RIGHTWARDS_ARROW_FROM_BAR: Rel = rel('↦', RelCategory::Default);
// pub const DOWNWARDS_ARROW_FROM_BAR: Rel = rel('↧', RelCategory::Default);
// pub const UP_DOWN_ARROW_WITH_BASE: Rel = rel('↨', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_HOOK: Rel = rel('↩', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_HOOK: Rel = rel('↪', RelCategory::Default);
pub const LEFTWARDS_ARROW_WITH_LOOP: Rel = rel('↫', RelCategory::Default);
pub const RIGHTWARDS_ARROW_WITH_LOOP: Rel = rel('↬', RelCategory::Default);
pub const LEFT_RIGHT_WAVE_ARROW: Rel = rel('↭', RelCategory::Default);
pub const LEFT_RIGHT_ARROW_WITH_STROKE: Rel = rel('↮', RelCategory::Default);
pub const DOWNWARDS_ZIGZAG_ARROW: Rel = rel('↯', RelCategory::Default);
pub const UPWARDS_ARROW_WITH_TIP_LEFTWARDS: Rel = rel('↰', RelCategory::Default);
pub const UPWARDS_ARROW_WITH_TIP_RIGHTWARDS: Rel = rel('↱', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_TIP_LEFTWARDS: Rel = rel('↲', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_TIP_RIGHTWARDS: Rel = rel('↳', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_CORNER_DOWNWARDS: Rel = rel('↴', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_CORNER_LEFTWARDS: Rel = rel('↵', RelCategory::Default);
pub const ANTICLOCKWISE_TOP_SEMICIRCLE_ARROW: Rel = rel('↶', RelCategory::Default);
pub const CLOCKWISE_TOP_SEMICIRCLE_ARROW: Rel = rel('↷', RelCategory::Default);
// pub const NORTH_WEST_ARROW_TO_LONG_BAR: Rel = rel('↸', RelCategory::Default);
// pub const LEFTWARDS_ARROW_TO_BAR_OVER_RIGHTWARDS_ARROW_TO_BAR: Rel = rel('↹', RelCategory::Default);
pub const ANTICLOCKWISE_OPEN_CIRCLE_ARROW: Rel = rel('↺', RelCategory::Default);
pub const CLOCKWISE_OPEN_CIRCLE_ARROW: Rel = rel('↻', RelCategory::Default);
pub const LEFTWARDS_HARPOON_WITH_BARB_UPWARDS: Rel = rel('↼', RelCategory::Default);
pub const LEFTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Rel = rel('↽', RelCategory::Default);
pub const UPWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Rel = rel('↾', RelCategory::Default);
pub const UPWARDS_HARPOON_WITH_BARB_LEFTWARDS: Rel = rel('↿', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_WITH_BARB_UPWARDS: Rel = rel('⇀', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Rel = rel('⇁', RelCategory::Default);
pub const DOWNWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Rel = rel('⇂', RelCategory::Default);
pub const DOWNWARDS_HARPOON_WITH_BARB_LEFTWARDS: Rel = rel('⇃', RelCategory::Default);
pub const RIGHTWARDS_ARROW_OVER_LEFTWARDS_ARROW: Rel = rel('⇄', RelCategory::Default);
// pub const UPWARDS_ARROW_LEFTWARDS_OF_DOWNWARDS_ARROW: Rel = rel('⇅', RelCategory::Default);
pub const LEFTWARDS_ARROW_OVER_RIGHTWARDS_ARROW: Rel = rel('⇆', RelCategory::Default);
pub const LEFTWARDS_PAIRED_ARROWS: Rel = rel('⇇', RelCategory::Default);
pub const UPWARDS_PAIRED_ARROWS: Rel = rel('⇈', RelCategory::Default);
pub const RIGHTWARDS_PAIRED_ARROWS: Rel = rel('⇉', RelCategory::Default);
pub const DOWNWARDS_PAIRED_ARROWS: Rel = rel('⇊', RelCategory::Default);
pub const LEFTWARDS_HARPOON_OVER_RIGHTWARDS_HARPOON: Rel = rel('⇋', RelCategory::Default);
pub const RIGHTWARDS_HARPOON_OVER_LEFTWARDS_HARPOON: Rel = rel('⇌', RelCategory::Default);
pub const LEFTWARDS_DOUBLE_ARROW_WITH_STROKE: Rel = rel('⇍', RelCategory::Default);
pub const LEFT_RIGHT_DOUBLE_ARROW_WITH_STROKE: Rel = rel('⇎', RelCategory::Default);
pub const RIGHTWARDS_DOUBLE_ARROW_WITH_STROKE: Rel = rel('⇏', RelCategory::Default);
pub const LEFTWARDS_DOUBLE_ARROW: Rel = rel('⇐', RelCategory::Default);
pub const UPWARDS_DOUBLE_ARROW: Rel = rel('⇑', RelCategory::OnlyA);
pub const RIGHTWARDS_DOUBLE_ARROW: Rel = rel('⇒', RelCategory::Default);
pub const DOWNWARDS_DOUBLE_ARROW: Rel = rel('⇓', RelCategory::OnlyA);
pub const LEFT_RIGHT_DOUBLE_ARROW: Rel = rel('⇔', RelCategory::Default);
pub const UP_DOWN_DOUBLE_ARROW: Rel = rel('⇕', RelCategory::OnlyA);
// pub const NORTH_WEST_DOUBLE_ARROW: Rel = rel('⇖', RelCategory::Default);
// pub const NORTH_EAST_DOUBLE_ARROW: Rel = rel('⇗', RelCategory::Default);
// pub const SOUTH_EAST_DOUBLE_ARROW: Rel = rel('⇘', RelCategory::Default);
// pub const SOUTH_WEST_DOUBLE_ARROW: Rel = rel('⇙', RelCategory::Default);
pub const LEFTWARDS_TRIPLE_ARROW: Rel = rel('⇚', RelCategory::Default);
pub const RIGHTWARDS_TRIPLE_ARROW: Rel = rel('⇛', RelCategory::Default);
// pub const LEFTWARDS_SQUIGGLE_ARROW: Rel = rel('⇜', RelCategory::Default);
pub const RIGHTWARDS_SQUIGGLE_ARROW: Rel = rel('⇝', RelCategory::Default);
// pub const UPWARDS_ARROW_WITH_DOUBLE_STROKE: Rel = rel('⇞', RelCategory::Default);
// pub const DOWNWARDS_ARROW_WITH_DOUBLE_STROKE: Rel = rel('⇟', RelCategory::Default);
// pub const LEFTWARDS_DASHED_ARROW: Rel = rel('⇠', RelCategory::Default);
// pub const UPWARDS_DASHED_ARROW: Rel = rel('⇡', RelCategory::Default);
// pub const RIGHTWARDS_DASHED_ARROW: Rel = rel('⇢', RelCategory::Default);
// pub const DOWNWARDS_DASHED_ARROW: Rel = rel('⇣', RelCategory::Default);
// pub const LEFTWARDS_ARROW_TO_BAR: Rel = rel('⇤', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_TO_BAR: Rel = rel('⇥', RelCategory::Default);
// pub const LEFTWARDS_WHITE_ARROW: Rel = rel('⇦', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW: Rel = rel('⇧', RelCategory::Default);
// pub const RIGHTWARDS_WHITE_ARROW: Rel = rel('⇨', RelCategory::Default);
// pub const DOWNWARDS_WHITE_ARROW: Rel = rel('⇩', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_FROM_BAR: Rel = rel('⇪', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL: Rel = rel('⇫', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_HORIZONTAL_BAR: Rel = rel('⇬', RelCategory::Default);
// pub const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_VERTICAL_BAR: Rel = rel('⇭', RelCategory::Default);
// pub const UPWARDS_WHITE_DOUBLE_ARROW: Rel = rel('⇮', RelCategory::Default);
// pub const UPWARDS_WHITE_DOUBLE_ARROW_ON_PEDESTAL: Rel = rel('⇯', RelCategory::Default);
// pub const RIGHTWARDS_WHITE_ARROW_FROM_WALL: Rel = rel('⇰', RelCategory::Default);
// pub const NORTH_WEST_ARROW_TO_CORNER: Rel = rel('⇱', RelCategory::Default);
// pub const SOUTH_EAST_ARROW_TO_CORNER: Rel = rel('⇲', RelCategory::Default);
// pub const UP_DOWN_WHITE_ARROW: Rel = rel('⇳', RelCategory::Default);
// pub const RIGHT_ARROW_WITH_SMALL_CIRCLE: Rel = rel('⇴', RelCategory::Default);
// pub const DOWNWARDS_ARROW_LEFTWARDS_OF_UPWARDS_ARROW: Rel = rel('⇵', RelCategory::Default);
// pub const THREE_RIGHTWARDS_ARROWS: Rel = rel('⇶', RelCategory::Default);
// pub const LEFTWARDS_ARROW_WITH_VERTICAL_STROKE: Rel = rel('⇷', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_VERTICAL_STROKE: Rel = rel('⇸', RelCategory::Default);
// pub const LEFT_RIGHT_ARROW_WITH_VERTICAL_STROKE: Rel = rel('⇹', RelCategory::Default);
// pub const LEFTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = rel('⇺', RelCategory::Default);
// pub const RIGHTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = rel('⇻', RelCategory::Default);
// pub const LEFT_RIGHT_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Rel = rel('⇼', RelCategory::Default);
// pub const LEFTWARDS_OPEN_HEADED_ARROW: Rel = rel('⇽', RelCategory::Default);
// pub const RIGHTWARDS_OPEN_HEADED_ARROW: Rel = rel('⇾', RelCategory::Default);
// pub const LEFT_RIGHT_OPEN_HEADED_ARROW: Rel = rel('⇿', RelCategory::Default);

//
// Unicode Block: Mathematical Operators
//
pub const FOR_ALL: OrdLike = ord('∀', OrdCategory::OnlyD);
pub const COMPLEMENT: OrdLike = ord('∁', OrdCategory::OnlyD);
pub const PARTIAL_DIFFERENTIAL: char = '∂'; // char so that it can be transformed
pub const THERE_EXISTS: OrdLike = ord('∃', OrdCategory::OnlyD);
pub const THERE_DOES_NOT_EXIST: OrdLike = ord('∄', OrdCategory::OnlyD);
pub const EMPTY_SET: char = '∅';
// pub const INCREMENT: Ord = ord('∆');
pub const NABLA: char = '∇'; // char so that it can be transformed
pub const ELEMENT_OF: Rel = rel('∈', RelCategory::Default);
pub const NOT_AN_ELEMENT_OF: Rel = rel('∉', RelCategory::Default);
// pub const SMALL_ELEMENT_OF: Rel = rel('∊', RelCategory::Default);
pub const CONTAINS_AS_MEMBER: Rel = rel('∋', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_MEMBER: Rel = rel('∌', RelCategory::Default);
// pub const SMALL_CONTAINS_AS_MEMBER: Rel = rel('∍', RelCategory::Default);
// pub const END_OF_PROOF: Ord = ord('∎');
pub const N_ARY_PRODUCT: BigOp = big_op('∏', BigOpCategory::OnlyJ);
pub const N_ARY_COPRODUCT: BigOp = big_op('∐', BigOpCategory::OnlyJ);
pub const N_ARY_SUMMATION: BigOp = big_op('∑', BigOpCategory::OnlyJ);
pub const MINUS_SIGN: Bin = bin('−', BinCategory::BD);
pub const MINUS_OR_PLUS_SIGN: Bin = bin('∓', BinCategory::BD);
pub const DOT_PLUS: Bin = bin('∔', BinCategory::BD);
// pub const DIVISION_SLASH: Op = Op('∕');
pub const SET_MINUS: Bin = bin('∖', BinCategory::BD);
pub const ASTERISK_OPERATOR: Bin = bin('∗', BinCategory::BD);
pub const RING_OPERATOR: Bin = bin('∘', BinCategory::OnlyC);
pub const BULLET_OPERATOR: Bin = bin('∙', BinCategory::OnlyC);
// pub const SQUARE_ROOT: Op = Op('√');
// pub const CUBE_ROOT: Op = Op('∛');
// pub const FOURTH_ROOT: Op = Op('∜');
pub const PROPORTIONAL_TO: Rel = rel('∝', RelCategory::Default);
pub const INFINITY: char = '∞';
// pub const RIGHT_ANGLE: Op = Op('∟');
pub const ANGLE: char = '∠';
pub const MEASURED_ANGLE: char = '∡';
pub const SPHERICAL_ANGLE: char = '∢';
pub const DIVIDES: Rel = rel('∣', RelCategory::Default);
pub const DOES_NOT_DIVIDE: Rel = rel('∤', RelCategory::Default);
pub const PARALLEL_TO: Rel = rel('∥', RelCategory::Default);
pub const NOT_PARALLEL_TO: Rel = rel('∦', RelCategory::Default);
pub const LOGICAL_AND: Bin = bin('∧', BinCategory::BD);
pub const LOGICAL_OR: Bin = bin('∨', BinCategory::BD);
pub const INTERSECTION: Bin = bin('∩', BinCategory::BD);
pub const UNION: Bin = bin('∪', BinCategory::BD);
pub const INTEGRAL: BigOp = big_op('∫', BigOpCategory::OnlyH);
pub const DOUBLE_INTEGRAL: BigOp = big_op('∬', BigOpCategory::OnlyH);
pub const TRIPLE_INTEGRAL: BigOp = big_op('∭', BigOpCategory::OnlyH);
pub const CONTOUR_INTEGRAL: BigOp = big_op('∮', BigOpCategory::OnlyH);
pub const SURFACE_INTEGRAL: BigOp = big_op('∯', BigOpCategory::OnlyH);
pub const VOLUME_INTEGRAL: BigOp = big_op('∰', BigOpCategory::OnlyH);
pub const CLOCKWISE_INTEGRAL: BigOp = big_op('∱', BigOpCategory::OnlyH);
pub const CLOCKWISE_CONTOUR_INTEGRAL: BigOp = big_op('∲', BigOpCategory::OnlyH);
pub const ANTICLOCKWISE_CONTOUR_INTEGRAL: BigOp = big_op('∳', BigOpCategory::OnlyH);
pub const THEREFORE: Rel = rel('∴', RelCategory::Default);
pub const BECAUSE: Rel = rel('∵', RelCategory::Default);
// pub const RATIO: Op = Op('∶');
pub const PROPORTION: Rel = rel('∷', RelCategory::Default);
// pub const DOT_MINUS: Op = Op('∸');
pub const EXCESS: Rel = rel('∹', RelCategory::Default);
pub const GEOMETRIC_PROPORTION: Rel = rel('∺', RelCategory::Default);
pub const HOMOTHETIC: Rel = rel('∻', RelCategory::Default);
pub const TILDE_OPERATOR: Rel = rel('∼', RelCategory::Default);
pub const REVERSED_TILDE: Rel = rel('∽', RelCategory::Default);
// pub const INVERTED_LAZY_S: Op = Op('∾');
// pub const SINE_WAVE: Op = Op('∿');
pub const WREATH_PRODUCT: Bin = bin('≀', BinCategory::OnlyC);
pub const NOT_TILDE: Rel = rel('≁', RelCategory::Default);
pub const MINUS_TILDE: Rel = rel('≂', RelCategory::Default);
pub const ASYMPTOTICALLY_EQUAL_TO: Rel = rel('≃', RelCategory::Default);
pub const NOT_ASYMPTOTICALLY_EQUAL_TO: Rel = rel('≄', RelCategory::Default);
pub const APPROXIMATELY_EQUAL_TO: Rel = rel('≅', RelCategory::Default);
// pub const APPROXIMATELY_BUT_NOT_ACTUALLY_EQUAL_TO: Rel = rel('≆', RelCategory::Default);
// pub const NEITHER_APPROXIMATELY_NOR_ACTUALLY_EQUAL_TO: Rel = rel('≇', RelCategory::Default);
pub const ALMOST_EQUAL_TO: Rel = rel('≈', RelCategory::Default);
pub const NOT_ALMOST_EQUAL_TO: Rel = rel('≉', RelCategory::Default);
pub const ALMOST_EQUAL_OR_EQUAL_TO: Rel = rel('≊', RelCategory::Default);
// pub const TRIPLE_TILDE: Rel = rel('≋', RelCategory::Default);
// pub const ALL_EQUAL_TO: Rel = rel('≌', RelCategory::Default);
pub const EQUIVALENT_TO: Rel = rel('≍', RelCategory::Default);
pub const GEOMETRICALLY_EQUIVALENT_TO: Rel = rel('≎', RelCategory::Default);
pub const DIFFERENCE_BETWEEN: Rel = rel('≏', RelCategory::Default);
pub const APPROACHES_THE_LIMIT: Rel = rel('≐', RelCategory::Default);
pub const GEOMETRICALLY_EQUAL_TO: Rel = rel('≑', RelCategory::Default);
pub const APPROXIMATELY_EQUAL_TO_OR_THE_IMAGE_OF: Rel = rel('≒', RelCategory::Default);
pub const IMAGE_OF_OR_APPROXIMATELY_EQUAL_TO: Rel = rel('≓', RelCategory::Default);
pub const COLON_EQUALS: Rel = rel('≔', RelCategory::Default);
pub const EQUALS_COLON: Rel = rel('≕', RelCategory::Default);
pub const RING_IN_EQUAL_TO: Rel = rel('≖', RelCategory::Default);
pub const RING_EQUAL_TO: Rel = rel('≗', RelCategory::Default);
pub const CORRESPONDS_TO: Rel = rel('≘', RelCategory::Default);
pub const ESTIMATES: Rel = rel('≙', RelCategory::Default);
pub const EQUIANGULAR_TO: Rel = rel('≚', RelCategory::Default);
pub const STAR_EQUALS: Rel = rel('≛', RelCategory::Default);
pub const DELTA_EQUAL_TO: Rel = rel('≜', RelCategory::Default);
pub const EQUAL_TO_BY_DEFINITION: Rel = rel('≝', RelCategory::Default);
pub const MEASURED_BY: Rel = rel('≞', RelCategory::Default);
pub const QUESTIONED_EQUAL_TO: Rel = rel('≟', RelCategory::Default);
pub const NOT_EQUAL_TO: Rel = rel('≠', RelCategory::Default);
pub const IDENTICAL_TO: Rel = rel('≡', RelCategory::Default);
pub const NOT_IDENTICAL_TO: Rel = rel('≢', RelCategory::Default);
// pub const STRICTLY_EQUIVALENT_TO: Rel = rel('≣', RelCategory::Default);
pub const LESS_THAN_OR_EQUAL_TO: Rel = rel('≤', RelCategory::Default);
pub const GREATER_THAN_OR_EQUAL_TO: Rel = rel('≥', RelCategory::Default);
pub const LESS_THAN_OVER_EQUAL_TO: Rel = rel('≦', RelCategory::Default);
pub const GREATER_THAN_OVER_EQUAL_TO: Rel = rel('≧', RelCategory::Default);
pub const LESS_THAN_BUT_NOT_EQUAL_TO: Rel = rel('≨', RelCategory::Default);
pub const GREATER_THAN_BUT_NOT_EQUAL_TO: Rel = rel('≩', RelCategory::Default);
pub const MUCH_LESS_THAN: Rel = rel('≪', RelCategory::Default);
pub const MUCH_GREATER_THAN: Rel = rel('≫', RelCategory::Default);
pub const BETWEEN: Rel = rel('≬', RelCategory::Default);
// pub const NOT_EQUIVALENT_TO: Rel = rel('≭', RelCategory::Default);
pub const NOT_LESS_THAN: Rel = rel('≮', RelCategory::Default);
pub const NOT_GREATER_THAN: Rel = rel('≯', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_EQUAL_TO: Rel = rel('≰', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_EQUAL_TO: Rel = rel('≱', RelCategory::Default);
pub const LESS_THAN_OR_EQUIVALENT_TO: Rel = rel('≲', RelCategory::Default);
pub const GREATER_THAN_OR_EQUIVALENT_TO: Rel = rel('≳', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_EQUIVALENT_TO: Rel = rel('≴', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_EQUIVALENT_TO: Rel = rel('≵', RelCategory::Default);
pub const LESS_THAN_OR_GREATER_THAN: Rel = rel('≶', RelCategory::Default);
pub const GREATER_THAN_OR_LESS_THAN: Rel = rel('≷', RelCategory::Default);
pub const NEITHER_LESS_THAN_NOR_GREATER_THAN: Rel = rel('≸', RelCategory::Default);
pub const NEITHER_GREATER_THAN_NOR_LESS_THAN: Rel = rel('≹', RelCategory::Default);
pub const PRECEDES: Rel = rel('≺', RelCategory::Default);
pub const SUCCEEDS: Rel = rel('≻', RelCategory::Default);
pub const PRECEDES_OR_EQUAL_TO: Rel = rel('≼', RelCategory::Default);
pub const SUCCEEDS_OR_EQUAL_TO: Rel = rel('≽', RelCategory::Default);
pub const PRECEDES_OR_EQUIVALENT_TO: Rel = rel('≾', RelCategory::Default);
pub const SUCCEEDS_OR_EQUIVALENT_TO: Rel = rel('≿', RelCategory::Default);
pub const DOES_NOT_PRECEDE: Rel = rel('⊀', RelCategory::Default);
pub const DOES_NOT_SUCCEED: Rel = rel('⊁', RelCategory::Default);
pub const SUBSET_OF: Rel = rel('⊂', RelCategory::Default);
pub const SUPERSET_OF: Rel = rel('⊃', RelCategory::Default);
pub const NOT_A_SUBSET_OF: Rel = rel('⊄', RelCategory::Default);
pub const NOT_A_SUPERSET_OF: Rel = rel('⊅', RelCategory::Default);
pub const SUBSET_OF_OR_EQUAL_TO: Rel = rel('⊆', RelCategory::Default);
pub const SUPERSET_OF_OR_EQUAL_TO: Rel = rel('⊇', RelCategory::Default);
pub const NEITHER_A_SUBSET_OF_NOR_EQUAL_TO: Rel = rel('⊈', RelCategory::Default);
pub const NEITHER_A_SUPERSET_OF_NOR_EQUAL_TO: Rel = rel('⊉', RelCategory::Default);
pub const SUBSET_OF_WITH_NOT_EQUAL_TO: Rel = rel('⊊', RelCategory::Default);
pub const SUPERSET_OF_WITH_NOT_EQUAL_TO: Rel = rel('⊋', RelCategory::Default);
// pub const MULTISET: Bin = bin('⊌', BinCategory::BD);
// pub const MULTISET_MULTIPLICATION: Bin = bin('⊍', BinCategory::BD);
pub const MULTISET_UNION: Bin = bin('⊎', BinCategory::BD);
pub const SQUARE_IMAGE_OF: Rel = rel('⊏', RelCategory::Default);
pub const SQUARE_ORIGINAL_OF: Rel = rel('⊐', RelCategory::Default);
pub const SQUARE_IMAGE_OF_OR_EQUAL_TO: Rel = rel('⊑', RelCategory::Default);
pub const SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Rel = rel('⊒', RelCategory::Default);
pub const SQUARE_CAP: Bin = bin('⊓', BinCategory::BD);
pub const SQUARE_CUP: Bin = bin('⊔', BinCategory::BD);
pub const CIRCLED_PLUS: Bin = bin('⊕', BinCategory::BD);
pub const CIRCLED_MINUS: Bin = bin('⊖', BinCategory::BD);
pub const CIRCLED_TIMES: Bin = bin('⊗', BinCategory::OnlyC);
pub const CIRCLED_DIVISION_SLASH: Bin = bin('⊘', BinCategory::BD);
pub const CIRCLED_DOT_OPERATOR: Bin = bin('⊙', BinCategory::OnlyC);
pub const CIRCLED_RING_OPERATOR: Bin = bin('⊚', BinCategory::OnlyC);
pub const CIRCLED_ASTERISK_OPERATOR: Bin = bin('⊛', BinCategory::OnlyC);
// pub const CIRCLED_EQUALS: Op = Op('⊜');
pub const CIRCLED_DASH: Bin = bin('⊝', BinCategory::BD);
pub const SQUARED_PLUS: Bin = bin('⊞', BinCategory::BD);
pub const SQUARED_MINUS: Bin = bin('⊟', BinCategory::BD);
pub const SQUARED_TIMES: Bin = bin('⊠', BinCategory::OnlyC);
pub const SQUARED_DOT_OPERATOR: Bin = bin('⊡', BinCategory::OnlyC);
pub const RIGHT_TACK: Rel = rel('⊢', RelCategory::Default);
pub const LEFT_TACK: Rel = rel('⊣', RelCategory::Default);
pub const DOWN_TACK: char = '⊤';
pub const UP_TACK: char = '⊥';
// pub const ASSERTION: Op = Op('⊦');
// pub const MODELS: Op = Op('⊧');
pub const TRUE: Rel = rel('⊨', RelCategory::Default);
pub const FORCES: Rel = rel('⊩', RelCategory::Default);
pub const TRIPLE_VERTICAL_BAR_RIGHT_TURNSTILE: Rel = rel('⊪', RelCategory::Default);
pub const DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Rel = rel('⊫', RelCategory::Default);
pub const DOES_NOT_PROVE: Rel = rel('⊬', RelCategory::Default);
pub const NOT_TRUE: Rel = rel('⊭', RelCategory::Default);
pub const DOES_NOT_FORCE: Rel = rel('⊮', RelCategory::Default);
pub const NEGATED_DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Rel = rel('⊯', RelCategory::Default);
// pub const PRECEDES_UNDER_RELATION: Op = Op('⊰');
// pub const SUCCEEDS_UNDER_RELATION: Op = Op('⊱');
pub const NORMAL_SUBGROUP_OF: Rel = rel('⊲', RelCategory::Default);
pub const CONTAINS_AS_NORMAL_SUBGROUP: Rel = rel('⊳', RelCategory::Default);
pub const NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Rel = rel('⊴', RelCategory::Default);
pub const CONTAINS_AS_NORMAL_SUBGROUP_OR_EQUAL_TO: Rel = rel('⊵', RelCategory::Default);
// pub const ORIGINAL_OF: Op = Op('⊶');
// pub const IMAGE_OF: Op = Op('⊷');
pub const MULTIMAP: Rel = rel('⊸', RelCategory::Default);
// pub const HERMITIAN_CONJUGATE_MATRIX: Op = Op('⊹');
pub const INTERCALATE: Rel = rel('⊺', RelCategory::Default);
pub const XOR: Bin = bin('⊻', BinCategory::BD);
pub const NAND: Bin = bin('⊼', BinCategory::BD);
// pub const NOR: Op = Op('⊽');
// pub const RIGHT_ANGLE_WITH_ARC: Op = Op('⊾');
// pub const RIGHT_TRIANGLE: Op = Op('⊿');
pub const N_ARY_LOGICAL_AND: BigOp = big_op('⋀', BigOpCategory::OnlyJ);
pub const N_ARY_LOGICAL_OR: BigOp = big_op('⋁', BigOpCategory::OnlyJ);
pub const N_ARY_INTERSECTION: BigOp = big_op('⋂', BigOpCategory::OnlyJ);
pub const N_ARY_UNION: BigOp = big_op('⋃', BigOpCategory::OnlyJ);
pub const DIAMOND_OPERATOR: Bin = bin('⋄', BinCategory::OnlyC);
// pub const DOT_OPERATOR: Bin = bin('⋅', BinCategory::BD);
pub const STAR_OPERATOR: Bin = bin('⋆', BinCategory::OnlyC);
pub const DIVISION_TIMES: Bin = bin('⋇', BinCategory::OnlyC);
pub const BOWTIE: Rel = rel('⋈', RelCategory::Default);
pub const LEFT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Bin = bin('⋉', BinCategory::OnlyC);
pub const RIGHT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Bin = bin('⋊', BinCategory::OnlyC);
pub const LEFT_SEMIDIRECT_PRODUCT: Bin = bin('⋋', BinCategory::OnlyC);
pub const RIGHT_SEMIDIRECT_PRODUCT: Bin = bin('⋌', BinCategory::OnlyC);
pub const REVERSED_TILDE_EQUALS: Rel = rel('⋍', RelCategory::Default);
pub const CURLY_LOGICAL_OR: Bin = bin('⋎', BinCategory::BD);
pub const CURLY_LOGICAL_AND: Bin = bin('⋏', BinCategory::BD);
pub const DOUBLE_SUBSET: Rel = rel('⋐', RelCategory::Default);
pub const DOUBLE_SUPERSET: Rel = rel('⋑', RelCategory::Default);
pub const DOUBLE_INTERSECTION: Bin = bin('⋒', BinCategory::BD);
pub const DOUBLE_UNION: Bin = bin('⋓', BinCategory::BD);
pub const PITCHFORK: Rel = rel('⋔', RelCategory::Default);
// pub const EQUAL_AND_PARALLEL_TO: Op = Op('⋕');
pub const LESS_THAN_WITH_DOT: Rel = rel('⋖', RelCategory::Default);
// pub const GREATER_THAN_WITH_DOT: Rel = rel('⋗', RelCategory::Default);
pub const VERY_MUCH_LESS_THAN: Rel = rel('⋘', RelCategory::Default);
// pub const VERY_MUCH_GREATER_THAN: Rel = rel('⋙', RelCategory::Default);
pub const LESS_THAN_EQUAL_TO_OR_GREATER_THAN: Rel = rel('⋚', RelCategory::Default);
pub const GREATER_THAN_EQUAL_TO_OR_LESS_THAN: Rel = rel('⋛', RelCategory::Default);
// pub const EQUAL_TO_OR_LESS_THAN: Rel = rel('⋜', RelCategory::Default);
// pub const EQUAL_TO_OR_GREATER_THAN: Rel = rel('⋝', RelCategory::Default);
pub const EQUAL_TO_OR_PRECEDES: Rel = rel('⋞', RelCategory::Default);
pub const EQUAL_TO_OR_SUCCEEDS: Rel = rel('⋟', RelCategory::Default);
pub const DOES_NOT_PRECEDE_OR_EQUAL: Rel = rel('⋠', RelCategory::Default);
pub const DOES_NOT_SUCCEED_OR_EQUAL: Rel = rel('⋡', RelCategory::Default);
// pub const NOT_SQUARE_IMAGE_OF_OR_EQUAL_TO: Rel = rel('⋢', RelCategory::Default);
// pub const NOT_SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Rel = rel('⋣', RelCategory::Default);
// pub const SQUARE_IMAGE_OF_OR_NOT_EQUAL_TO: Rel = rel('⋤', RelCategory::Default);
// pub const SQUARE_ORIGINAL_OF_OR_NOT_EQUAL_TO: Rel = rel('⋥', RelCategory::Default);
pub const LESS_THAN_BUT_NOT_EQUIVALENT_TO: Rel = rel('⋦', RelCategory::Default);
pub const GREATER_THAN_BUT_NOT_EQUIVALENT_TO: Rel = rel('⋧', RelCategory::Default);
pub const PRECEDES_BUT_NOT_EQUIVALENT_TO: Rel = rel('⋨', RelCategory::Default);
pub const SUCCEEDS_BUT_NOT_EQUIVALENT_TO: Rel = rel('⋩', RelCategory::Default);
// pub const NOT_NORMAL_SUBGROUP_OF: Rel = rel('⋪', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP: Rel = rel('⋫', RelCategory::Default);
// pub const NOT_NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Rel = rel('⋬', RelCategory::Default);
// pub const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP_OR_EQUAL: Rel = rel('⋭', RelCategory::Default);
pub const VERTICAL_ELLIPSIS: Rel = rel('⋮', RelCategory::Default);
// pub const MIDLINE_HORIZONTAL_ELLIPSIS: Rel = rel('⋯', RelCategory::Default);
// pub const UP_RIGHT_DIAGONAL_ELLIPSIS: Op = Op('⋰');
pub const DOWN_RIGHT_DIAGONAL_ELLIPSIS: Rel = rel('⋱', RelCategory::Default);
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
pub const LEFT_CEILING: Fence = Fence('⌈');
pub const RIGHT_CEILING: Fence = Fence('⌉');
pub const LEFT_FLOOR: Fence = Fence('⌊');
pub const RIGHT_FLOOR: Fence = Fence('⌋');
pub const TOP_LEFT_CORNER: char = '⌜';
pub const TOP_RIGHT_CORNER: char = '⌝';
pub const BOTTOM_LEFT_CORNER: char = '⌞';
pub const BOTTOM_RIGHT_CORNER: char = '⌟';
pub const FROWN: Rel = rel('⌢', RelCategory::Default);
pub const SMILE: Rel = rel('⌣', RelCategory::Default);
pub const TOP_SQUARE_BRACKET: OrdLike = ord('⎴', OrdCategory::OnlyI);
pub const BOTTOM_SQUARE_BRACKET: OrdLike = ord('⎵', OrdCategory::OnlyI);
pub const TOP_PARENTHESIS: OrdLike = ord('⏜', OrdCategory::OnlyI);
pub const BOTTOM_PARENTHESIS: OrdLike = ord('⏝', OrdCategory::OnlyI);
pub const TOP_CURLY_BRACKET: OrdLike = ord('⏞', OrdCategory::OnlyI);
pub const BOTTOM_CURLY_BRACKET: OrdLike = ord('⏟', OrdCategory::OnlyI);

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
pub const PERPENDICULAR: Rel = rel('⟂', RelCategory::Default);

pub const MATHEMATICAL_LEFT_WHITE_SQUARE_BRACKET: Fence = Fence('⟦');
pub const MATHEMATICAL_RIGHT_WHITE_SQUARE_BRACKET: Fence = Fence('⟧');
pub const MATHEMATICAL_LEFT_ANGLE_BRACKET: Fence = Fence('⟨');
pub const MATHEMATICAL_RIGHT_ANGLE_BRACKET: Fence = Fence('⟩');
pub const MATHEMATICAL_LEFT_DOUBLE_ANGLE_BRACKET: Fence = Fence('⟪');
pub const MATHEMATICAL_RIGHT_DOUBLE_ANGLE_BRACKET: Fence = Fence('⟫');
// pub const MATHEMATICAL_LEFT_WHITE_TORTOISE_SHELL_BRACKET: Fence = Fence('⟬');
// pub const MATHEMATICAL_RIGHT_WHITE_TORTOISE_SHELL_BRACKET: Fence = Fence('⟭');
pub const MATHEMATICAL_LEFT_FLATTENED_PARENTHESIS: Fence = Fence('⟮');
pub const MATHEMATICAL_RIGHT_FLATTENED_PARENTHESIS: Fence = Fence('⟯');

//
// Unicode Block: Supplemental Arrows-A
//
pub const LONG_LEFTWARDS_ARROW: Rel = rel('⟵', RelCategory::Default);
pub const LONG_RIGHTWARDS_ARROW: Rel = rel('⟶', RelCategory::Default);
pub const LONG_LEFT_RIGHT_ARROW: Rel = rel('⟷', RelCategory::Default);
pub const LONG_LEFTWARDS_DOUBLE_ARROW: Rel = rel('⟸', RelCategory::Default);
pub const LONG_RIGHTWARDS_DOUBLE_ARROW: Rel = rel('⟹', RelCategory::Default);
pub const LONG_LEFT_RIGHT_DOUBLE_ARROW: Rel = rel('⟺', RelCategory::Default);
// pub const LONG_LEFTWARDS_ARROW_FROM_BAR: Op = Op('⟻');
pub const LONG_RIGHTWARDS_ARROW_FROM_BAR: Rel = rel('⟼', RelCategory::Default);

//
// Unicode Block: Supplemental Arrows-B
//
pub const LEFTWARDS_ARROW_TAIL: Rel = rel('⤙', RelCategory::Default);
pub const RIGHTWARDS_ARROW_TAIL: Rel = rel('⤚', RelCategory::Default);

//
// Unicode Block: Miscellaneous Mathematical Symbols-B
//
pub const LEFT_WHITE_CURLY_BRACKET: Fence = Fence('⦃');
pub const RIGHT_WHITE_CURLY_BRACKET: Fence = Fence('⦄');
// pub const LEFT_WHITE_PARENTHESIS: Fence = fence('⦅', Stretchy::Always);
// pub const RIGHT_WHITE_PARENTHESIS: Fence = fence('⦆', false, Stretchy::Always);
pub const Z_NOTATION_LEFT_IMAGE_BRACKET: Fence = Fence('⦇');
pub const Z_NOTATION_RIGHT_IMAGE_BRACKET: Fence = Fence('⦈');
pub const Z_NOTATION_LEFT_BINDING_BRACKET: Fence = Fence('⦉');
pub const Z_NOTATION_RIGHT_BINDING_BRACKET: Fence = Fence('⦊');

pub const SQUARED_RISING_DIAGONAL_SLASH: Bin = bin('⧄', BinCategory::BD);
pub const SQUARED_FALLING_DIAGONAL_SLASH: Bin = bin('⧅', BinCategory::BD);
pub const SQUARED_SQUARE: Bin = bin('⧈', BinCategory::BD);
pub const BLACK_LOZENGE: char = '⧫';

// pub const REVERSE_SOLIDUS_OPERATOR: Bin = bin('⧵', BinCategory::BD);

//
// Unicode Block: Supplemental Mathematical Operators
//
pub const N_ARY_CIRCLED_DOT_OPERATOR: BigOp = big_op('⨀', BigOpCategory::OnlyJ);
pub const N_ARY_CIRCLED_PLUS_OPERATOR: BigOp = big_op('⨁', BigOpCategory::OnlyJ);
pub const N_ARY_CIRCLED_TIMES_OPERATOR: BigOp = big_op('⨂', BigOpCategory::OnlyJ);
pub const N_ARY_UNION_OPERATOR_WITH_DOT: BigOp = big_op('⨃', BigOpCategory::OnlyJ);
pub const N_ARY_UNION_OPERATOR_WITH_PLUS: BigOp = big_op('⨄', BigOpCategory::OnlyJ);
pub const N_ARY_SQUARE_INTERSECTION_OPERATOR: BigOp = big_op('⨅', BigOpCategory::OnlyJ);
pub const N_ARY_SQUARE_UNION_OPERATOR: BigOp = big_op('⨆', BigOpCategory::OnlyJ);
pub const TWO_LOGICAL_AND_OPERATOR: BigOp = big_op('⨇', BigOpCategory::OnlyJ);
pub const TWO_LOGICAL_OR_OPERATOR: BigOp = big_op('⨈', BigOpCategory::OnlyJ);
pub const N_ARY_TIMES_OPERATOR: BigOp = big_op('⨉', BigOpCategory::OnlyJ);
// pub const MODULO_TWO_SUM: Op = Op('⨊');
pub const SUMMATION_WITH_INTEGRAL: BigOp = big_op('⨋', BigOpCategory::OnlyH);
pub const QUADRUPLE_INTEGRAL_OPERATOR: BigOp = big_op('⨌', BigOpCategory::OnlyH);
pub const FINITE_PARTL_INTEGRAL: BigOp = big_op('⨍', BigOpCategory::OnlyH);
pub const INTEGRAL_WITH_DOUBLE_STROKE: BigOp = big_op('⨎', BigOpCategory::OnlyH);
pub const INTEGRAL_AVERAGE_WITH_SLASH: BigOp = big_op('⨏', BigOpCategory::OnlyH);
pub const CIRCULATION_FUNCTION: BigOp = big_op('⨐', BigOpCategory::OnlyH);
pub const ANTICLOCKWISE_INTEGRATION: BigOp = big_op('⨑', BigOpCategory::OnlyH);
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
pub const Z_NOTATION_SCHEMA_COMPOSITION: Rel = rel('⨟', RelCategory::Default);
// pub const Z_NOTATION_SCHEMA_PIPING: Op = Op('⨠');
// pub const Z_NOTATION_SCHEMA_PROJECTION: Op = Op('⨡');
// pub const PLUS_SIGN_WITH_SMALL_CIRCLE_ABOVE: Bin = bin('⨢', BinCategory::BD);
// pub const PLUS_SIGN_WITH_CIRCUMFLEX_ACCENT_ABOVE: Bin = bin('⨣', BinCategory::BD);
// pub const PLUS_SIGN_WITH_TILDE_ABOVE: Bin = bin('⨤', BinCategory::BD);
// pub const PLUS_SIGN_WITH_DOT_BELOW: Bin = bin('⨥', BinCategory::BD);
// pub const PLUS_SIGN_WITH_TILDE_BELOW: Bin = bin('⨦', BinCategory::BD);
// pub const PLUS_SIGN_WITH_SUBSCRIPT_TWO: Bin = bin('⨧', BinCategory::BD);
// pub const PLUS_SIGN_WITH_BLACK_TRIANGLE: Bin = bin('⨨', BinCategory::BD);
// pub const MINUS_SIGN_WITH_COMMA_ABOVE: Bin = bin('⨩', BinCategory::BD);
// pub const MINUS_SIGN_WITH_DOT_BELOW: Bin = bin('⨪', BinCategory::BD);
// pub const MINUS_SIGN_WITH_FALLING_DOTS: Bin = bin('⨫', BinCategory::BD);
// pub const MINUS_SIGN_WITH_RISING_DOTS: Bin = bin('⨬', BinCategory::BD);
// pub const PLUS_SIGN_IN_LEFT_HALF_CIRCLE: Bin = bin('⨭', BinCategory::BD);
// pub const PLUS_SIGN_IN_RIGHT_HALF_CIRCLE: Bin = bin('⨮', BinCategory::BD);
// pub const VECTOR_OR_CROSS_PRODUCT: Bin = bin('⨯', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_WITH_DOT_ABOVE: Bin = bin('⨰', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_WITH_UNDERBAR: Bin = bin('⨱', BinCategory::BD);
// pub const SEMIDIRECT_PRODUCT_WITH_BOTTOM_CLOSED: Bin = bin('⨲', BinCategory::BD);
// pub const SMASH_PRODUCT: Bin = bin('⨳', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_IN_LEFT_HALF_CIRCLE: Bin = bin('⨴', BinCategory::BD);
// pub const MULTIPLICATION_SIGN_IN_RIGHT_HALF_CIRCLE: Bin = bin('⨵', BinCategory::BD);
// pub const CIRCLED_MULTIPLICATION_SIGN_WITH_CIRCUMFLEX_ACCENT: Op = Op('⨶');
// pub const MULTIPLICATION_SIGN_IN_DOUBLE_CIRCLE: Op = Op('⨷');
// pub const CIRCLED_DIVISION_SIGN: Op = Op('⨸');
// pub const PLUS_SIGN_IN_TRIANGLE: Op = Op('⨹');
// pub const MINUS_SIGN_IN_TRIANGLE: Op = Op('⨺');
// pub const MULTIPLICATION_SIGN_IN_TRIANGLE: Op = Op('⨻');
// pub const INTERIOR_PRODUCT: Op = Op('⨼');
// pub const RIGHTHAND_INTERIOR_PRODUCT: Op = Op('⨽');
// pub const Z_NOTATION_RELATIONAL_COMPOSITION: Op = Op('⨾');
pub const AMALGAMATION_OR_COPRODUCT: Rel = rel('⨿', RelCategory::Default);
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
pub const LOGICAL_AND_WITH_DOUBLE_OVERBAR: Rel = rel('⩞', RelCategory::Default);
// pub const LOGICAL_AND_WITH_UNDERBAR: Op = Op('⩟');
// pub const LOGICAL_AND_WITH_DOUBLE_UNDERBAR: Op = Op('⩠');
// pub const SMALL_VEE_WITH_UNDERBAR: Op = Op('⩡');
// pub const LOGICAL_OR_WITH_DOUBLE_OVERBAR: Op = Op('⩢');
// pub const LOGICAL_OR_WITH_DOUBLE_UNDERBAR: Op = Op('⩣');
// pub const Z_NOTATION_DOMAIN_ANTIRESTRICTION: Op = Op('⩤');
// pub const Z_NOTATION_RANGE_ANTIRESTRICTION: Op = Op('⩥');
pub const EQUALS_SIGN_WITH_DOT_BELOW: Rel = rel('⩦', RelCategory::Default);
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
pub const LESS_THAN_OR_SLANTED_EQUAL_TO: Rel = rel('⩽', RelCategory::Default);
pub const GREATER_THAN_OR_SLANTED_EQUAL_TO: Rel = rel('⩾', RelCategory::Default);
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⩿');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⪀');
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪁');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪂');
// pub const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_RIGHT: Op = Op('⪃');
// pub const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_LEFT: Op = Op('⪄');
pub const LESS_THAN_OR_APPROXIMATE: Rel = rel('⪅', RelCategory::Default);
pub const GREATER_THAN_OR_APPROXIMATE: Rel = rel('⪆', RelCategory::Default);
pub const LESS_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Rel = rel('⪇', RelCategory::Default);
pub const GREATER_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Rel = rel('⪈', RelCategory::Default);
pub const LESS_THAN_AND_NOT_APPROXIMATE: Rel = rel('⪉', RelCategory::Default);
pub const GREATER_THAN_AND_NOT_APPROXIMATE: Rel = rel('⪊', RelCategory::Default);
pub const LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_GREATER_THAN: Rel =
    rel('⪋', RelCategory::Default);
pub const GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_LESS_THAN: Rel =
    rel('⪌', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SIMILAR_OR_EQUAL: Rel = rel('⪍', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SIMILAR_OR_EQUAL: Rel = rel('⪎', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SIMILAR_ABOVE_GREATER_THAN: Rel = rel('⪏', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SIMILAR_ABOVE_LESS_THAN: Rel = rel('⪐', RelCategory::Default);
// pub const LESS_THAN_ABOVE_GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL: Rel = rel('⪑', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL: Rel = rel('⪒', RelCategory::Default);
// pub const LESS_THAN_ABOVE_SLANTED_EQUAL_ABOVE_GREATER_THAN_ABOVE_SLANTED_EQUAL: Rel = rel('⪓', RelCategory::Default);
// pub const GREATER_THAN_ABOVE_SLANTED_EQUAL_ABOVE_LESS_THAN_ABOVE_SLANTED_EQUAL: Rel = rel('⪔', RelCategory::Default);
pub const SLANTED_EQUAL_TO_OR_LESS_THAN: Rel = rel('⪕', RelCategory::Default);
pub const SLANTED_EQUAL_TO_OR_GREATER_THAN: Rel = rel('⪖', RelCategory::Default);
// pub const SLANTED_EQUAL_TO_OR_LESS_THAN_WITH_DOT_INSIDE: Rel = rel('⪗', RelCategory::Default);
// pub const SLANTED_EQUAL_TO_OR_GREATER_THAN_WITH_DOT_INSIDE: Rel = rel('⪘', RelCategory::Default);
// pub const DOUBLE_LINE_EQUAL_TO_OR_LESS_THAN: Rel = rel('⪙', RelCategory::Default);
// pub const DOUBLE_LINE_EQUAL_TO_OR_GREATER_THAN: Rel = rel('⪚', RelCategory::Default);
// pub const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_LESS_THAN: Rel = rel('⪛', RelCategory::Default);
// pub const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_GREATER_THAN: Rel = rel('⪜', RelCategory::Default);
// pub const SIMILAR_OR_LESS_THAN: Rel = rel('⪝', RelCategory::Default);
// pub const SIMILAR_OR_GREATER_THAN: Rel = rel('⪞', RelCategory::Default);
// pub const SIMILAR_ABOVE_LESS_THAN_ABOVE_EQUALS_SIGN: Rel = rel('⪟', RelCategory::Default);
// pub const SIMILAR_ABOVE_GREATER_THAN_ABOVE_EQUALS_SIGN: Rel = rel('⪠', RelCategory::Default);
// pub const DOUBLE_NESTED_LESS_THAN: Rel = rel('⪡', RelCategory::Default);
// pub const DOUBLE_NESTED_GREATER_THAN: Rel = rel('⪢', RelCategory::Default);
// pub const DOUBLE_NESTED_LESS_THAN_WITH_UNDERBAR: Rel = rel('⪣', RelCategory::Default);
// pub const GREATER_THAN_OVERLAPPING_LESS_THAN: Rel = rel('⪤', RelCategory::Default);
// pub const GREATER_THAN_BESIDE_LESS_THAN: Rel = rel('⪥', RelCategory::Default);
// pub const LESS_THAN_CLOSED_BY_CURVE: Rel = rel('⪦', RelCategory::Default);
// pub const GREATER_THAN_CLOSED_BY_CURVE: Rel = rel('⪧', RelCategory::Default);
// pub const LESS_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Rel = rel('⪨', RelCategory::Default);
// pub const GREATER_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Rel = rel('⪩', RelCategory::Default);
// pub const SMALLER_THAN: Rel = rel('⪪', RelCategory::Default);
// pub const LARGER_THAN: Rel = rel('⪫', RelCategory::Default);
// pub const SMALLER_THAN_OR_EQUAL_TO: Rel = rel('⪬', RelCategory::Default);
// pub const LARGER_THAN_OR_EQUAL_TO: Rel = rel('⪭', RelCategory::Default);
// pub const EQUALS_SIGN_WITH_BUMPY_ABOVE: Rel = rel('⪮', RelCategory::Default);
pub const PRECEDES_ABOVE_SINGLE_LINE_EQUALS_SIGN: Rel = rel('⪯', RelCategory::Default);
pub const SUCCEEDS_ABOVE_SINGLE_LINE_EQUALS_SIGN: Rel = rel('⪰', RelCategory::Default);
// pub const PRECEDES_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Rel = rel('⪱', RelCategory::Default);
// pub const SUCCEEDS_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Rel = rel('⪲', RelCategory::Default);
// pub const PRECEDES_ABOVE_EQUALS_SIGN: Rel = rel('⪳', RelCategory::Default);
// pub const SUCCEEDS_ABOVE_EQUALS_SIGN: Rel = rel('⪴', RelCategory::Default);
pub const PRECEDES_ABOVE_NOT_EQUAL_TO: Rel = rel('⪵', RelCategory::Default);
pub const SUCCEEDS_ABOVE_NOT_EQUAL_TO: Rel = rel('⪶', RelCategory::Default);
pub const PRECEDES_ABOVE_ALMOST_EQUAL_TO: Rel = rel('⪷', RelCategory::Default);
pub const SUCCEEDS_ABOVE_ALMOST_EQUAL_TO: Rel = rel('⪸', RelCategory::Default);
pub const PRECEDES_ABOVE_NOT_ALMOST_EQUAL_TO: Rel = rel('⪹', RelCategory::Default);
pub const SUCCEEDS_ABOVE_NOT_ALMOST_EQUAL_TO: Rel = rel('⪺', RelCategory::Default);
// pub const DOUBLE_PRECEDES: Rel = rel('⪻', RelCategory::Default);
// pub const DOUBLE_SUCCEEDS: Rel = rel('⪼', RelCategory::Default);
// pub const SUBSET_WITH_DOT: Rel = rel('⪽', RelCategory::Default);
// pub const SUPERSET_WITH_DOT: Rel = rel('⪾', RelCategory::Default);
// pub const SUBSET_WITH_PLUS_SIGN_BELOW: Rel = rel('⪿', RelCategory::Default);
// pub const SUPERSET_WITH_PLUS_SIGN_BELOW: Rel = rel('⫀', RelCategory::Default);
// pub const SUBSET_WITH_MULTIPLICATION_SIGN_BELOW: Rel = rel('⫁', RelCategory::Default);
// pub const SUPERSET_WITH_MULTIPLICATION_SIGN_BELOW: Rel = rel('⫂', RelCategory::Default);
// pub const SUBSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Rel = rel('⫃', RelCategory::Default);
// pub const SUPERSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Rel = rel('⫄', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_EQUALS_SIGN: Rel = rel('⫅', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_EQUALS_SIGN: Rel = rel('⫆', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_TILDE_OPERATOR: Rel = rel('⫇', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_TILDE_OPERATOR: Rel = rel('⫈', RelCategory::Default);
// pub const SUBSET_OF_ABOVE_ALMOST_EQUAL_TO: Rel = rel('⫉', RelCategory::Default);
// pub const SUPERSET_OF_ABOVE_ALMOST_EQUAL_TO: Rel = rel('⫊', RelCategory::Default);
pub const SUBSET_OF_ABOVE_NOT_EQUAL_TO: Rel = rel('⫋', RelCategory::Default);
pub const SUPERSET_OF_ABOVE_NOT_EQUAL_TO: Rel = rel('⫌', RelCategory::Default);

//
// Unicode Block: Small Form Variants
//
// pub const SMALL_REVERSE_SOLIDUS: Rel = rel('﹨', RelCategory::Default);
