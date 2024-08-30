use std::mem::discriminant;

use strum_macros::AsRefStr;

use crate::attribute::{FracAttr, MathVariant, ParenAttr, Style, TextTransform};
use crate::ops::Op;

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr)]
pub enum Token<'source> {
    #[strum(serialize = "end of document")]
    EOF(u16),
    #[strum(serialize = r"\begin{...}")]
    Begin(u16),
    #[strum(serialize = r"\end{...}")]
    End(u16),
    #[strum(serialize = "&")]
    Ampersand(u16),
    #[strum(serialize = r"\\")]
    NewLine(u16),
    #[strum(serialize = r"\left")]
    Left(u16),
    #[strum(serialize = r"\right")]
    Right(u16),
    #[strum(serialize = r"\middle")]
    Middle(u16),
    #[strum(serialize = "parenthesis")]
    Paren(Op, Option<ParenAttr>, u16),
    /// The closing square bracket has its own token because we often
    /// need to search for it.
    #[strum(serialize = "]")]
    SquareBracketClose(u16),
    #[strum(serialize = "{")]
    GroupBegin(u16),
    #[strum(serialize = "}")]
    GroupEnd(u16),
    Frac(Option<FracAttr>, u16),
    #[strum(serialize = r"\genfrac")]
    Genfrac(u16),
    #[strum(serialize = "_")]
    Underscore(u16),
    #[strum(serialize = "^")]
    Circumflex(u16),
    Binom(Option<FracAttr>, u16),
    #[strum(serialize = r"\overset")]
    Overset(u16),
    #[strum(serialize = r"\underset")]
    Underset(u16),
    Overbrace(Op, u16),
    Underbrace(Op, u16),
    #[strum(serialize = r"\sqrt")]
    Sqrt(u16),
    Integral(Op, u16),
    #[strum(serialize = r"\limits")]
    Limits(u16),
    Lim(&'static str, u16),
    Space(&'static str, u16),
    #[strum(serialize = "~")]
    NonBreakingSpace(u16),
    Whitespace(u16),
    Transform(Option<TextTransform>, Option<MathVariant>, u16),
    Big(&'static str, u16),
    Over(Op, u16),
    Under(Op, u16),
    Operator(Op, u16),
    #[strum(serialize = "'")]
    Prime(u16),
    #[strum(serialize = ">")]
    OpGreaterThan(u16),
    #[strum(serialize = "<")]
    OpLessThan(u16),
    #[strum(serialize = r"\&")]
    OpAmpersand(u16),
    #[strum(serialize = ":")]
    Colon(u16),
    BigOp(Op, u16),
    Letter(char, u16),
    NormalLetter(char, u16), // letter for which we need `mathvariant="normal"`
    Number(&'source str, u16),
    NumberWithDot(&'source str, u16),
    NumberWithComma(&'source str, u16),
    Function(&'static str, u16),
    #[strum(serialize = r"\operatorname")]
    OperatorName(u16),
    Slashed(u16),
    #[strum(serialize = r"\not")]
    Not(u16),
    #[strum(serialize = r"\text*")]
    Text(Option<TextTransform>, u16),
    #[strum(serialize = r"\mathstrut")]
    Mathstrut(u16),
    Style(Style, u16),
    UnknownCommand(&'source str, u16),
}

impl Token<'_> {
    pub(crate) fn acts_on_a_digit(&self) -> bool {
        matches!(
            self,
            Token::Sqrt(_)
                | Token::Frac(_, _)
                | Token::Binom(_, _)
                | Token::Transform(Some(_), None, _)
        )
    }

    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind(&self, other: &Token) -> bool {
        discriminant(self) == discriminant(other)
    }
}
