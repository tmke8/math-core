use std::mem::discriminant;
use std::num::NonZeroU8;

use strum_macros::IntoStaticStr;

use super::character_class::Class;
use super::environments::Env;

use crate::mathml_renderer::attribute::{
    FracAttr, MathVariant, Notation, OpAttr, Size, Style, TextTransform,
};
use crate::mathml_renderer::length::Length;
use crate::mathml_renderer::symbol::{BigOp, Bin, Fence, MathMLOperator, OrdLike, Punct, Rel};

#[derive(Debug, Clone, Copy, IntoStaticStr)]
#[repr(u32)]
pub enum Token<'source> {
    #[strum(serialize = "end of document")]
    Eof,
    #[strum(serialize = r"\begin")]
    Begin,
    #[strum(serialize = r"\end")]
    End,
    /// For `\begin{...}` and `\end{...}`.
    EnvName(Env),
    #[strum(serialize = "&")]
    NewColumn,
    #[strum(serialize = r"\\")]
    NewLine,
    #[strum(serialize = r"\nonumber/\notag")]
    NoNumber,
    #[strum(serialize = r"\tag")]
    Tag,
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
    /// A token to force an operator to behave like a relation (mathrel).
    /// This is, for example, needed for `:`, which in LaTeX is a relation,
    /// but in MathML Core is a separator (punctuation).
    ForceRelation(MathMLOperator),
    /// A token to force an operator to behave like a closing symbol (mathclose).
    /// This is, for example, needed for `!`, which in LaTeX is a closing symbol,
    /// but in MathML Core is an ordinary operator.
    ForceClose(MathMLOperator),
    Letter(char),
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Digit(char),
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
    CustomCmd(u8, &'source [Token<'static>]),
    HardcodedMathML(&'static str),
    TextModeAccent(char),
    StringLiteral(&'source str),
    StoredStringLiteral(usize, usize),
}

impl Token<'_> {
    /// Returns `true` if `self` and `other` are of the same kind.
    /// Note that this does not compare the content of the tokens.
    pub(crate) fn is_same_kind_as(&self, other: &Token) -> bool {
        discriminant(self) == discriminant(other)
    }

    /// Returns the character class of this token.
    pub(super) fn class(&self, in_sequence: bool, ignore_end_tokens: bool) -> Class {
        if !in_sequence {
            return Class::Default;
        }
        match self {
            Token::Relation(_) | Token::ForceRelation(_) => Class::Relation,
            Token::Punctuation(_) => Class::Punctuation,
            Token::Open(_) | Token::Left | Token::SquareBracketOpen => Class::Open,
            Token::Close(_)
            | Token::SquareBracketClose
            | Token::NewColumn
            | Token::ForceClose(_) => Class::Close,
            Token::BinaryOp(_) => Class::BinaryOp,
            Token::BigOp(_) | Token::Integral(_) => Class::Operator,
            Token::End | Token::Right | Token::GroupEnd | Token::Eof if !ignore_end_tokens => {
                Class::Close
            }
            Token::Big(_, Some(cls)) => *cls,
            Token::CustomCmd(_, [head, ..]) => head.class(in_sequence, ignore_end_tokens),
            _ => Class::Default,
        }
    }

    #[inline]
    pub(super) fn needs_string_literal(&self) -> Option<NonZeroU8> {
        match self {
            Token::CustomSpace | Token::Color | Token::Tag => NonZeroU8::new(1),
            Token::Genfrac => NonZeroU8::new(3),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

    #[inline]
    pub(super) fn class(&self, in_sequence: bool, ignore_end_tokens: bool) -> Class {
        self.1.class(in_sequence, ignore_end_tokens)
    }
}

impl<'source> From<Token<'source>> for TokLoc<'source> {
    #[inline]
    fn from(token: Token<'source>) -> Self {
        TokLoc(0, token)
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
            std::mem::size_of::<TokLoc>() <= 4 * WORD,
            "size of TokResult"
        );
        assert!(
            std::mem::size_of::<Result<Token, &'static i32>>() <= 3 * WORD,
            "size of Result<Token, pointer>"
        );
    }
}
