use std::mem::discriminant;

use mathml_renderer::ast::Node;
use mathml_renderer::attribute::{FracAttr, MathVariant, OpAttr, Size, Style, TextTransform};
use mathml_renderer::symbol::{Big, Bin, Op, ParenOp, Rel};
use strum_macros::AsRefStr;

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
    Delimiter(&'static ParenOp),
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
    Integral(Big),
    #[strum(serialize = r"\limits")]
    Limits,
    Lim(&'static str),
    Space(&'static str),
    #[strum(serialize = "~")]
    NonBreakingSpace,
    Whitespace,
    Transform(MathVariant),
    Big(Size),
    OverUnder(Op, bool, Option<OpAttr>),
    Relation(Rel),
    #[strum(serialize = "binary operator")]
    BinaryOp(Bin),
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
    BigOp(Big),
    Letter(char),
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Number(Digit),
    Function(&'static str),
    #[strum(serialize = r"\operatorname")]
    OperatorName,
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text*")]
    Text(Option<TextTransform>),
    Style(Style),
    Color,
    CustomCmd(usize, &'static Node<'static>),
    GetCollectedLetters,
    HardcodedMathML(&'static str),
    UnknownCommand(&'source str),
}

impl Token<'_> {
    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind_as(&self, other: &Token) -> bool {
        discriminant(self) == discriminant(other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Digit {
    Zero = b'0',
    One = b'1',
    Two = b'2',
    Three = b'3',
    Four = b'4',
    Five = b'5',
    Six = b'6',
    Seven = b'7',
    Eight = b'8',
    Nine = b'9',
}

impl TryFrom<char> for Digit {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        if value.is_ascii_digit() {
            // Safety:
            // 1. We've verified this is an ASCII digit ('0'..='9')
            // 2. Digit is #[repr(u8)] with variants exactly matching ASCII values
            // 3. The input char is converted to the exact matching byte value
            Ok(unsafe { std::mem::transmute::<u8, Digit>(value as u8) })
        } else {
            Err(())
        }
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
