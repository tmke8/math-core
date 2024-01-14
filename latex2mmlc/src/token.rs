use strum_macros::AsRefStr;

use crate::attribute::{DisplayStyle, Style, TextTransform};
use crate::ops::Op;

#[derive(Debug, Clone, PartialEq, AsRefStr)]
pub enum Token<'a> {
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
    #[strum(serialize = "{")]
    GroupBegin,
    #[strum(serialize = "}")]
    GroupEnd,
    Frac(Option<DisplayStyle>),
    #[strum(serialize = r"\genfrac")]
    Genfrac,
    #[strum(serialize = "_")]
    Underscore,
    #[strum(serialize = "^")]
    Circumflex,
    Binom(Option<DisplayStyle>),
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
    Number(&'a str, Op),
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
    UnknownCommand(&'a str),
}

impl<'a> Token<'a> {
    pub(crate) fn acts_on_a_digit(&self) -> bool {
        matches!(
            self,
            Token::Sqrt | Token::Frac(_) | Token::Binom(_) | Token::Transform(_)
        )
    }
}
