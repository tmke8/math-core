use std::ops::Range;

use strum_macros::IntoStaticStr;

use mathml_renderer::attribute::{
    FracAttr, HtmlTextStyle, MathVariant, Notation, OpAttr, ParenType, Size, Style,
};
use mathml_renderer::length::Length;
use mathml_renderer::symbol::{Bin, MathMLOperator, Op, OrdLike, Punct, Rel};

use crate::character_class::Class;
use crate::environments::Env;

#[derive(Debug, Clone, Copy)]
pub enum Token<'source> {
    /// End of input.
    Eoi,
    /// The beginning of an environment, e.g. `\begin{matrix}`.
    Begin(Env),
    /// The end of an environment, e.g. `\end{matrix}`.
    End(Env),
    /// A new column in an array or matrix, e.g. `&` in `\begin{matrix} a & b\\c & d \end{matrix}`.
    NewColumn,
    /// A new line in an array or matrix, e.g. `\\` in `\begin{matrix} a & b\\c & d \end{matrix}`.
    NewLine,
    /// Suppresses numbering for the current equation.
    NoNumber,
    /// A tag for the current equation.
    Tag,
    /// A left delimiter, e.g. `\left(`.
    Left,
    /// A right delimiter, e.g. `\right)`.
    Right,
    /// A middle delimiter, e.g. `\middle|`.
    Middle,
    /// The character `[`. It has its own token because we need to
    /// distinguish it from `\lbrack` after, e.g., `\sqrt`.
    SquareBracketOpen,
    /// The character `]`. It has its own token because we often need to search for it.
    /// Additionally, it's useful to distinguish this from `\rbrack`.
    SquareBracketClose,
    /// The character `{`.
    GroupBegin,
    /// The character `}`.
    GroupEnd,
    /// A token for `\frac` and `\cfrac`, `\dfrac` and `\tfrac`. The `Option<FracAttr>` is `None`
    /// for `\frac` and, for example, `Some(FracAttr::DisplayStyleTrue)` for `\dfrac`.
    Frac(Option<FracAttr>),
    /// `\genfrac`
    Genfrac,
    /// The character `_` for subscripts.
    Underscore,
    /// The character `^` for superscripts.
    Circumflex,
    /// A token for `\binom`, `\dbinom` and `\tbinom`. The `Option<FracAttr>` is `None` for
    /// `\binom` and, for example, `Some(FracAttr::DisplayStyleTrue)` for `\dbinom`.
    Binom(Option<FracAttr>),
    /// `\overset`
    Overset,
    /// `\underset`
    Underset,
    /// `\overbrace` and `\underbrace`. The `bool` is `true` for overbraces and `false` for
    /// underbraces.
    OverUnderBrace(OrdLike, bool),
    /// `\sqrt` and `\sqrt[n]{...}`
    Sqrt,
    /// `\limits`
    Limits,
    /// Fixed-length spaces, e.g. `\,`, `\;`, `\quad`, etc.
    Space(Length),
    /// A custom space specified by the user, e.g. `\hspace{1em}`.
    CustomSpace,
    /// A non-breaking space, e.g. `~`.
    NonBreakingSpace,
    /// A whitespace character, e.g. ` `.
    Whitespace,
    /// A token for transforming to a specific math variant, e.g. `\mathbf`.
    Transform(MathVariant),
    /// A token for switching the math variant, e.g. `\bf`.
    TransformSwitch(MathVariant),
    /// A sized parenthesis, e.g. `\bigl(`, `\Biggr)`.
    Big(Size, Option<ParenType>),
    /// Stretchy and non-stretchy accents, e.g. `\hat`, `\widehat`, `\bar`, `\overline`, etc.
    /// The `bool` is `true` for over-accents and `false` for under-accents.
    Accent(Rel, bool, Option<OpAttr>),
    /// A token corresponding to LaTeX's "mathord" character class (class 0).
    Ord(OrdLike),
    /// A token corresponding to LaTeX's "mathop" character class (class 1).
    Op(Op),
    /// A token corresponding to LaTeX's "mathbin" character class (class 2).
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
    /// The character `'`.
    Prime,
    /// The character `>`.
    /// It has its own token because we need to escape it for the HTML output.
    OpGreaterThan,
    /// The character `<`.
    /// It has its own token because we need to escape it for the HTML output.
    OpLessThan,
    /// The character `&`.
    /// It has its own token because we need to escape it for the HTML output.
    OpAmpersand,
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
    /// `\mathbin`
    Mathbin,
    /// A token to allow a relation to stretch.
    /// Used in, e.g., `\xrightarrow` and `\xleftarrow`.
    StretchyRel(Rel),
    /// An ordinary letter, e.g. `a`, `b`, `c`.
    Letter(char),
    /// A letter for which we need `mathvariant="normal"`.
    /// For example, upper-case greek letter like `\Gamma`, which should be rendered upright.
    UprightLetter(char),
    /// A digit, e.g. `0`, `1`, `2`.
    Digit(char),
    /// Text-based operators without limits.
    /// For example, `\log`, `\exp`, `\sin`, `\cos`, `\tan`.
    PseudoOperator(&'static str),
    /// Text-based operators with limits.
    /// For example, `\lim`, `\sup`, `\inf`, `\max`, `\min`.
    PseudoOperatorLimits(&'static str),
    /// A token for enclosing notations, e.g. `\cancel`, `\xcancel`.
    Enclose(Notation),
    /// `\operatorname` and `\operatorname*`. The `bool` is `true` for `\operatorname*` and `false`
    /// for `\operatorname`.
    OperatorName(bool),
    /// `\slashed`
    Slashed,
    /// `\not`
    Not,
    /// A token for text, e.g. `\text{...}`, `\textit{...}`.
    Text(Option<HtmlTextStyle>),
    /// `\displaystyle`, `\textstyle`, `\scriptstyle` and `\scriptscriptstyle`.
    Style(Style),
    /// A token for math color, e.g. `\color{red}`.
    Color,
    /// A token used in custom commands defined by the user. The `u8` is the index of the argument,
    /// going from 0 to 8. For example, `\#1` corresponds to `CustomCmdArg(0)`.
    CustomCmdArg(u8),
    /// A token referencing a stream of tokens defined by the user. The `u8` is the number of
    /// arguments that the custom command takes.
    CustomCmd(u8, &'source [Token<'static>]),
    /// A token for hardcoded MathML. The `&'static str` is the MathML string to be inserted into
    /// the output.
    HardcodedMathML(&'static str),
    /// A token for text-mode accents, e.g. `\~{n}`. The `char` is a Unicode combining character,
    /// e.g. `\u{0303}` for the tilde accent.
    TextModeAccent(char),
    /// A token for unknown commands. This is used when `ignore_unknown_commands` is `true` in the
    /// configuration, and the parser encounters an unknown command. The `&'source str` is the name
    /// of the unknown command.
    UnknownCommand(&'source str),
    /// This token is intended to be used in predefined token streams.
    /// It is equivalent to `{abc}`, but has a much more compact representation.
    InternalStringLiteral(&'static str),
}

#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(Token<'_>, [usize; 3]);
#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(Result<Token<'_>, &'static i32>, [usize; 3]);

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
            Token::BinaryOp(_) | Token::ForceBinaryOp(_) | Token::Mathbin => Class::BinaryOp,
            Token::Op(_) => Class::Operator,
            Token::End(_) | Token::Right | Token::GroupEnd | Token::Eoi if !ignore_end_tokens => {
                Class::Close
            }
            Token::Inner(_) => Class::Inner,
            // `\big` commands without the "l" or "r" really produce `Class::Default`.
            Token::Big(_, Some(paren_type)) => {
                if matches!(paren_type, ParenType::Open) {
                    Class::Open
                } else {
                    Class::Close
                }
            }
            // TODO: This needs to skip spaces and other non-class tokens in the token sequence.
            Token::CustomCmd(_, [head, ..]) => head.class(in_sequence, ignore_end_tokens),
            _ => Class::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    #[inline]
    pub const fn zero_width(at: usize) -> Self {
        Span { start: at, end: at }
    }

    #[inline]
    pub const fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Returns a new `Span` with the same start position as `self`, but with the end position set
    /// to `self.start + length`.
    #[inline]
    pub const fn with_length(self, length: usize) -> Self {
        Span {
            start: self.start,
            end: self.start + length,
        }
    }
}

impl From<Span> for Range<usize> {
    #[inline]
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

/// A token together with its span in the input string.
#[derive(Debug, Clone, Copy)]
pub struct TokSpan<'config>(Token<'config>, Span);

#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(TokSpan<'_>, [usize; 5]);

impl<'config> TokSpan<'config> {
    #[inline]
    pub const fn new(token: Token<'config>, span: Span) -> Self {
        TokSpan(token, span)
    }

    #[inline]
    pub fn token(&self) -> &Token<'config> {
        &self.0
    }

    #[inline]
    pub fn into_token(self) -> Token<'config> {
        self.0
    }

    #[inline]
    pub fn into_parts(self) -> (Token<'config>, Span) {
        (self.0, self.1)
    }

    // #[inline]
    // pub fn token_mut(&mut self) -> &mut Token<'config> {
    //     &mut self.0
    // }

    #[inline]
    pub fn span(&self) -> Span {
        self.1
    }

    #[inline]
    pub(super) fn class(&self, in_sequence: bool, ignore_end_tokens: bool) -> Class {
        self.0.class(in_sequence, ignore_end_tokens)
    }
}

impl<'config> From<Token<'config>> for TokSpan<'config> {
    #[inline]
    fn from(token: Token<'config>) -> Self {
        TokSpan(token, Span::default())
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
    Eoi,
}

impl EndToken {
    pub fn matches(&self, other: &Token) -> bool {
        matches!(
            (self, other),
            (EndToken::End, Token::End(_))
                | (EndToken::GroupClose, Token::GroupEnd)
                | (EndToken::Right, Token::Right)
                | (EndToken::SquareBracketClose, Token::SquareBracketClose)
                | (EndToken::Eoi, Token::Eoi)
        )
    }
}
