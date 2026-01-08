use strum_macros::IntoStaticStr;

use mathml_renderer::attribute::{
    FracAttr, HtmlTextStyle, MathVariant, Notation, OpAttr, Size, Style,
};
use mathml_renderer::length::Length;
use mathml_renderer::symbol::{Bin, MathMLOperator, Op, OrdLike, Punct, Rel};

use crate::character_class::Class;
use crate::environments::Env;

#[derive(Debug, Clone, Copy, IntoStaticStr)]
pub enum Token<'config> {
    #[strum(serialize = "end of input")]
    Eof,
    #[strum(serialize = r"\begin")]
    Begin(Env),
    #[strum(serialize = r"\end")]
    End(Env),
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
    Big(Size, Option<Class>),
    OverUnder(Rel, bool, Option<OpAttr>),
    /// A token corresponding to LaTeX's "mathord" character class (class 0).
    Ord(OrdLike),
    /// A token corresponding to LaTeX's "mathop" character class (class 1).
    Op(Op),
    /// A token corresponding to LaTeX's "mathbin" character class (class 2).
    #[strum(serialize = "binary operator")]
    BinaryOp(Bin),
    /// A token corresponding to LaTeX's "mathrel" character class (class 3).
    Relation(Rel),
    /// A token corresponding to LaTeX's "mathopen" character class (class 4).
    Open(OrdLike),
    /// A token corresponding to LaTeX's "mathclose" character class (class 5).
    Close(OrdLike),
    /// A token corresponding to LaTeX's "mathpunct" character class (class 6).
    Punctuation(Punct),
    /// A token corresponding to LaTeX's "mathinner" character class (class I).
    Inner(Op),
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
    /// A token to force an operator to behave like a binary operator (mathbin).
    /// This is, for example, needed for `×`, which in LaTeX is a binary operator,
    /// but in MathML Core is a "big operator" (mathop).
    ForceBinaryOp(MathMLOperator),
    Letter(char, FromAscii),
    UprightLetter(char), // letter for which we need `mathvariant="normal"`
    Digit(char),
    // For `\log`, `\exp`, `\sin`, `\cos`, `\tan`, etc.
    PseudoOperator(&'static str),
    Enclose(Notation),
    #[strum(serialize = r"\operatorname")]
    OperatorName(bool),
    Slashed,
    #[strum(serialize = r"\not")]
    Not,
    #[strum(serialize = r"\text*")]
    Text(Option<HtmlTextStyle>),
    Style(Style),
    Color,
    CustomCmdArg(u8),
    CustomCmd(u8, &'config [Token<'static>]),
    HardcodedMathML(&'static str),
    TextModeAccent(char),
    /// This token is intended to be used in predefined token streams.
    /// It is equivalent to `{abc}`, but has a much more compact representation.
    InternalStringLiteral(&'static str),
}

impl Token<'_> {
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
            Token::Op(_) | Token::Integral(_) => Class::Operator,
            Token::End(_) | Token::Right | Token::GroupEnd | Token::Eof if !ignore_end_tokens => {
                Class::Close
            }
            Token::Inner(_) => Class::Inner,
            // `\big` commands without the "l" or "r" really produce `Class::Default`.
            Token::Big(_, Some(cls)) => *cls,
            // TODO: This needs to skip spaces and other non-class tokens in the token sequence.
            Token::CustomCmd(_, [head, ..]) => head.class(in_sequence, ignore_end_tokens),
            _ => Class::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum FromAscii {
    #[default]
    False,
    True,
}

#[derive(Debug, Clone, Copy)]
pub struct TokLoc<'config>(pub usize, pub Token<'config>);

impl<'config> TokLoc<'config> {
    #[inline]
    pub fn token(&self) -> &Token<'config> {
        &self.1
    }

    #[inline]
    pub fn into_token(self) -> Token<'config> {
        self.1
    }

    // #[inline]
    // pub fn token_mut(&mut self) -> &mut Token<'config> {
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

impl<'config> From<Token<'config>> for TokLoc<'config> {
    #[inline]
    fn from(token: Token<'config>) -> Self {
        TokLoc(0, token)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
pub enum EndToken {
    #[strum(serialize = r"\end{...}")]
    End,
    #[strum(serialize = r"}")]
    GroupClose,
    #[strum(serialize = r"\right")]
    Right,
    #[strum(serialize = r"]")]
    SquareBracketClose,
    #[strum(serialize = r"end of input")]
    Eof,
}

impl EndToken {
    pub fn matches(&self, other: &Token) -> bool {
        matches!(
            (self, other),
            (EndToken::End, Token::End(_))
                | (EndToken::GroupClose, Token::GroupEnd)
                | (EndToken::Right, Token::Right)
                | (EndToken::SquareBracketClose, Token::SquareBracketClose)
                | (EndToken::Eof, Token::Eof)
        )
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
