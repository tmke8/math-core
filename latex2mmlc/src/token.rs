use std::mem::discriminant;

use strum_macros::AsRefStr;

use crate::ast::Node;
use crate::attribute::{FracAttr, MathVariant, OpAttr, Size, Style, TextTransform};
use crate::ops::{Op, ParenOp};

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr)]
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
    #[strum(serialize = r"\middle")]
    Middle,
    #[strum(serialize = "parenthesis")]
    Paren(&'static ParenOp),
    /// The opening square bracket has its own token because we need to
    /// distinguish it from `\lbrack` after `\sqrt`.
    #[strum(serialize = "[")]
    SquareBracketOpen,
    /// The closing square bracket has its own token because we often
    /// need to search for it.
    /// Additionally, it's useful to distinguish this from `\rbrack`.
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
    #[strum(serialize = r"\overset")]
    Overset,
    #[strum(serialize = r"\underset")]
    Underset,
    OverUnderBrace(Op, bool),
    #[strum(serialize = r"\sqrt")]
    Sqrt,
    Integral(Op),
    #[strum(serialize = r"\limits")]
    Limits,
    Lim(&'static str),
    Space(&'static str),
    #[strum(serialize = "~")]
    NonBreakingSpace,
    Whitespace,
    SingleSpace,
    Transform(MathVariant),
    Big(Size),
    OverUnder(Op, bool, Option<OpAttr>),
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
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Number(&'source str),
    NumberWithDot(&'source str),
    NumberWithComma(&'source str),
    Function(&'static str),
    #[strum(serialize = r"\operatorname")]
    OperatorName,
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text*")]
    Text(Option<TextTransform>),
    #[strum(serialize = r"\mathstrut")]
    Mathstrut,
    Style(Style),
    CustomCmd0Args(&'static Node<'static>),
    CustomCmd1Arg(&'static Node<'static>),
    UnknownCommand(&'source str),
}

impl Token<'_> {
    pub(crate) fn acts_on_a_digit(&self) -> bool {
        matches!(
            self,
            Token::Sqrt
                | Token::Frac(_)
                | Token::Binom(_)
                | Token::Transform(MathVariant::Transform(_))
        )
    }

    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind(&self, other: &Token) -> bool {
        discriminant(self) == discriminant(other)
    }
}

#[derive(Debug)]
pub struct TokLoc<'source>(pub usize, pub Token<'source>);

impl<'source> TokLoc<'source> {
    #[inline]
    pub fn token(&self) -> &Token<'source> {
        &self.1
    }

    #[inline]
    pub fn into_token(self) -> Token<'source> {
        self.1
    }

    // #[inline]
    // pub fn token_mut(&mut self) -> &mut Token<'source> {
    //     &mut self.1
    // }

    #[inline]
    pub fn location(&self) -> usize {
        self.0
    }
}
