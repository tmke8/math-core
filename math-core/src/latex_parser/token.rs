use std::mem::discriminant;

use strum_macros::IntoStaticStr;

use crate::mathml_renderer::ast::Node;
use crate::mathml_renderer::attribute::{
    FracAttr, MathVariant, OpAttr, Size, Style, TextTransform,
};
use crate::mathml_renderer::length::Length;
use crate::mathml_renderer::symbol::{Bin, Fence, Op, Ord, Punct, Rel};

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
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
    Delimiter(&'static Fence),
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
    OverUnderBrace(Ord, bool),
    #[strum(serialize = r"\sqrt")]
    Sqrt,
    Integral(Op),
    #[strum(serialize = r"\limits")]
    Limits,
    // For `\lim`, `\sup`, `\inf`, `\max`, `\min`, etc.
    PseudoOperatorLimits(&'static str),
    Space(Length),
    CustomSpace,
    #[strum(serialize = "~")]
    NonBreakingSpace,
    Whitespace,
    Transform(MathVariant),
    Big(Size),
    OverUnder(Rel, bool, Option<OpAttr>),
    Relation(Rel),
    Ord(Ord),
    Punctuation(Punct),
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
    BigOp(Op),
    Letter(char),
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Number(Digit),
    // For `\log`, `\exp`, `\sin`, `\cos`, `\tan`, etc.
    PseudoOperator(&'source str),
    #[strum(serialize = r"\operatorname")]
    OperatorName,
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text*")]
    Text(Option<TextTransform>),
    Style(Style),
    Color,
    CustomCmd(usize, NodeRef<'source>),
    CustomCmdArg(usize),
    GetCollectedLetters,
    HardcodedMathML(&'static str),
    TextModeAccent(char),
    UnknownCommand(&'source str),
}

impl Token<'_> {
    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind_as(&self, other: &Token) -> bool {
        discriminant(self) == discriminant(other)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct NodeRef<'source>(&'source Node<'source>);

impl<'source> NodeRef<'source> {
    /// Creates a new `NodeRef` from a reference to a `Node`.
    pub(crate) const fn new(node: &'source Node<'source>) -> Self {
        NodeRef(node)
    }

    pub(crate) fn into_ref(self) -> &'source Node<'source> {
        self.0
    }
}

impl PartialEq for NodeRef<'_> {
    fn eq(&self, other: &NodeRef<'_>) -> bool {
        // Use pointer equality for NodeRef comparison.
        // This does the right thing for the equality for `Token`. Tokens can contain references
        // to pre-defined nodes and two tokens are equal if they refer to the same node.
        std::ptr::eq(self.0, other.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    const WORD: usize = std::mem::size_of::<usize>();

    #[test]
    fn test_struct_sizes() {
        assert!(std::mem::size_of::<Token>() <= 3 * WORD, "size of Token");
        assert!(std::mem::size_of::<TokLoc>() <= 4 * WORD, "size of TokLoc");
    }
}
