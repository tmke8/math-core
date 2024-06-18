use strum_macros::AsRefStr;

use crate::attribute::{FracAttr, Style, TextTransform};
use crate::ops::Op;

#[derive(Debug, Clone, PartialEq, AsRefStr)]
#[repr(u32)]
pub enum Token<'source> {
    #[strum(serialize = "end of document")]
    EOF,
    #[strum(serialize = r"\begin{...}")]
    Begin,
    #[strum(serialize = r"\end{...}")]
    End,
    #[strum(serialize = "&")]
    Ampersand,
    #[strum(serialize = r"\\")]
    NewLine,
    #[strum(serialize = r"\left")]
    Left,
    #[strum(serialize = r"\right")]
    Right,
    Middle,
    Paren(Op),
    #[strum(serialize = "]")]
    SquareBracketClose,
    #[strum(serialize = "{")]
    GroupBegin,
    #[strum(serialize = "}")]
    GroupEnd,
    Frac(Option<FracAttr>),
    #[strum(serialize = r"\genfrac")]
    Genfrac,
    #[strum(serialize = "_")]
    Underscore,
    #[strum(serialize = "^")]
    Circumflex,
    Binom(Option<FracAttr>),
    Overset,
    Underset,
    Overbrace(Op),
    Underbrace(Op),
    #[strum(serialize = r"\sqrt")]
    Sqrt,
    Integral(Op),
    #[strum(serialize = r"\limits")]
    Limits,
    Lim(&'static str),
    Space(&'static str),
    #[strum(serialize = "~")]
    NonBreakingSpace,
    Transform(TextTransform),
    NormalVariant,
    Big(&'static str),
    Over(Op),
    Under(Op),
    Operator(Op),
    #[strum(serialize = "'")]
    Prime,
    #[strum(serialize = ">")]
    OpGreaterThan,
    #[strum(serialize = "<")]
    OpLessThan,
    #[strum(serialize = r"\&")]
    OpAmpersand,
    #[strum(serialize = ":")]
    Colon,
    BigOp(Op),
    Letter(char),
    NormalLetter(char),
    Number(&'source str),
    NumberWithDot(&'source str),
    NumberWithComma(&'source str),
    Function(&'static str),
    #[strum(serialize = r"\operatorname")]
    OperatorName,
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text")]
    Text,
    #[strum(serialize = r"\mathstrut")]
    Mathstrut,
    Style(Style),
    UnknownCommand(&'source str),
}

impl Token<'_> {
    pub(crate) fn acts_on_a_digit(&self) -> bool {
        matches!(
            self,
            Token::Sqrt | Token::Frac(_) | Token::Binom(_) | Token::Transform(_)
        )
    }

    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind(&self, other: &Token) -> bool {
        // SAFETY: Because `Self` is marked `repr(u32)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u32` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        let self_discr = unsafe { *<*const _>::from(self).cast::<u32>() };
        let other_discr = unsafe { *<*const _>::from(other).cast::<u32>() };
        self_discr == other_discr
    }
}
