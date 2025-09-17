use std::mem::discriminant;

use strum_macros::IntoStaticStr;

use super::character_class::Class;
use super::environments::Env;
use super::error::LatexErrKind;

use crate::mathml_renderer::attribute::{
    FracAttr, MathVariant, Notation, OpAttr, Size, Style, TextTransform,
};
use crate::mathml_renderer::length::Length;
use crate::mathml_renderer::symbol::{BigOp, Bin, Fence, MathMLOperator, OrdLike, Punct, Rel};

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[repr(u32)]
pub enum Token<'config> {
    #[strum(serialize = "end of document")]
    Eof,
    #[strum(serialize = r"\begin{...}")]
    Begin(Env),
    #[strum(serialize = r"\end{...}")]
    End(Env),
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
    OverUnderBrace(OrdLike, bool),
    #[strum(serialize = r"\sqrt")]
    Sqrt,
    Integral(BigOp),
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
    Big(Size, Option<Class>),
    OverUnder(Rel, bool, Option<OpAttr>),
    /// A token corresponding to LaTeX's "mathord" character class (class 0).
    Ord(OrdLike),
    /// A token corresponding to LaTeX's "mathop" character class (class 1).
    BigOp(BigOp),
    /// A token corresponding to LaTeX's "mathbin" character class (class 2).
    #[strum(serialize = "binary operator")]
    BinaryOp(Bin),
    /// A token corresponding to LaTeX's "mathrel" character class (class 3).
    Relation(Rel),
    /// A token corresponding to LaTeX's "mathopen" character class (class 4).
    Open(Fence),
    /// A token corresponding to LaTeX's "mathclose" character class (class 5).
    Close(Fence),
    /// A token corresponding to LaTeX's "mathpunct" character class (class 6).
    Punctuation(Punct),
    #[strum(serialize = "'")]
    Prime,
    #[strum(serialize = ">")]
    OpGreaterThan,
    #[strum(serialize = "<")]
    OpLessThan,
    #[strum(serialize = r"\&")]
    OpAmpersand,
    #[strum(serialize = ":")]
    ForceRelation(MathMLOperator),
    Letter(char),
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Number(Digit),
    // For `\log`, `\exp`, `\sin`, `\cos`, `\tan`, etc.
    PseudoOperator(&'static str),
    Enclose(Notation),
    #[strum(serialize = r"\operatorname")]
    OperatorName,
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text*")]
    Text(Option<TextTransform>),
    Style(Style),
    Color,
    CustomCmdArg(u8),
    CustomCmd(u8, &'config [Token<'static>]),
    GetCollectedLetters,
    HardcodedMathML(&'static str),
    TextModeAccent(char),
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

pub(crate) type TokLoc<'config> = (usize, Token<'config>);

pub(crate) type ErrorTup<'arena, 'source> = (usize, &'arena LatexErrKind<'source>);

#[derive(Debug, Clone, Copy)]
pub struct TokResult<'arena, 'source>(
    pub usize,
    pub Result<Token<'source>, &'arena LatexErrKind<'source>>,
);

impl<'arena, 'source> TokResult<'arena, 'source> {
    #[inline]
    pub fn token(&self) -> &Result<Token<'source>, &'arena LatexErrKind<'source>> {
        &self.1
    }

    #[inline]
    pub fn into_token(self) -> Result<Token<'source>, &'arena LatexErrKind<'source>> {
        self.1
    }

    #[inline]
    pub fn with_error(self) -> Result<TokLoc<'source>, ErrorTup<'arena, 'source>> {
        match self.1 {
            Ok(tok) => Ok((self.0, tok)),
            Err(err_kind) => Err((self.0, err_kind)),
        }
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
        assert!(
            std::mem::size_of::<TokResult>() <= 4 * WORD,
            "size of TokResult"
        );
        assert!(
            std::mem::size_of::<Result<Token, &'static i32>>() <= 3 * WORD,
            "size of Result<Token, pointer>"
        );
    }
}
