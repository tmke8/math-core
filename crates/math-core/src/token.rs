use std::ops::Range;

use strum_macros::IntoStaticStr;

use mathml_renderer::symbol::{Bin, MathMLOperator, Op, OrdLike, Punct, Rel};
use mathml_renderer::{
    attribute::{FracAttr, HtmlTextStyle, Notation, OpAttrs, Size, Style},
    super_char::SuperChar,
};
use mathml_renderer::{length::Length, super_char::OverlayChar};

use crate::character_class::{Class, MathVariant, ParenType};
use crate::environments::Env;

#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// `\nonumber`/`\notag`, suppresses numbering for the current equation.
    NoNumber,
    /// `\tag`, tag for the current equation.
    Tag,
    /// `\label`, label for the current equation.
    Label,
    /// `\eqref`, equation reference to a label.
    EqRef,
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
    /// A token for `\over`, `\atop`, `\choose`, `\brace` and `\brack`.
    InfixGenFrac {
        with_line: bool,
        delim: Option<InfixDelim>,
    },
    /// `\genfrac`
    Genfrac,
    /// The character `_` for subscripts.
    Underscore,
    /// The character `^` for superscripts.
    Circumflex,
    /// `\prescript`
    Prescript,
    /// `\sideset`
    Sideset,
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
    Accent(MathMLOperator, bool, OpAttrs),
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
    /// A token to force an operator to behave like a binary operator (mathbin).
    /// This is, for example, needed for `×`, which in LaTeX is a binary operator,
    /// but in MathML Core is a "big operator" (mathop).
    ForceBinaryOp(MathMLOperator),
    /// A token to force an operator to behave like a relation (mathrel).
    /// This is, for example, needed for `:`, which in LaTeX is a relation,
    /// but in MathML Core is a separator (punctuation).
    ForceRelation(MathMLOperator),
    /// A token to force an operator to behave like an opening symbol (mathopen).
    ForceOpen(MathMLOperator, ForceStretchy),
    /// A token to force an operator to behave like a closing symbol (mathclose).
    /// This is, for example, needed for `!`, which in LaTeX is a closing symbol,
    /// but in MathML Core is an ordinary operator.
    ForceClose(MathMLOperator, ForceStretchy),
    /// A token to force an operator to behave like punctuation (mathpunct).
    ForcePunctuation(MathMLOperator),
    /// `\mathord` and `\mathbin`.
    MathClass(MathClassKind),
    /// A token for the extensible arrow commands `\xrightarrow`, `\xleftarrow`, etc.
    /// The `Rel` is the stretchy arrow operator to render.
    XArrow(Rel),
    /// An ordinary letter, e.g. `a`, `b`, `c`.
    Letter(SuperChar, Mode),
    /// A letter for which we need `mathvariant="normal"`.
    /// For example, upper-case greek letter like `\Gamma`, which should be rendered upright.
    UprightLetter(SuperChar),
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
    OperatorName { with_limits: bool },
    /// A combining overlay. `\not` uses U+0338, `\vertoverlay` uses U+20D2
    Overlay(OverlayChar),
    /// A token for text, e.g. `\text{...}`, `\textit{...}`.
    Text(Option<HtmlTextStyle>),
    /// `\displaystyle`, `\textstyle`, `\scriptstyle` and `\scriptscriptstyle`.
    Style(Style),
    /// `\cramped`.
    Cramped,
    /// A token for math color, e.g. `\color{red}`.
    Color,
    /// `\phantom`, `\hphantom`, or `\vphantom`
    Phantom(PhantomKind),
    /// A token used in custom commands defined by the user. The `u8` is the index of the argument,
    /// going from 0 to 8. For example, `\#1` corresponds to `CustomCmdArg(0)`.
    CustomCmdArg(u8),
    /// A token referencing a stream of tokens defined by the user. The `u8` is the number of
    /// arguments that the custom command takes.
    CustomCmd(u8, &'source [Token<'static>]),
    /// A token for text-mode accents, e.g. `\~{n}`. The `char` is a Unicode combining character,
    /// e.g. `\u{0303}` for the tilde accent.
    TextMode(TextToken),
    /// A token for commands that can be used in both math mode and text mode, e.g. `\{`. The `char`
    /// is the character that the command produces, e.g. `{` for `\{`.
    MathOrTextMode(&'static Token<'static>, char),
    /// A token for unknown commands. This is used when `ignore_unknown_commands` is `true` in the
    /// configuration, and the parser encounters an unknown command. The `&'source str` is the name
    /// of the unknown command.
    UnknownCommand(&'source str),
    /// This token is intended to be used in predefined token streams.
    /// It is equivalent to `{abc}`, but has a much more compact representation.
    InternalStringLiteral(&'static str),
}

/// The character class assigned by `\mathord` / `\mathbin` / `\mathopen` / `\mathclose`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathClassKind {
    /// `\mathord`: ordinary character class, with all spacing forced to zero.
    Ord,
    /// `\mathbin`: binary operator character class.
    Bin,
    /// `\mathopen`: opening delimiter character class, with all spacing forced to zero.
    Open,
    /// `\mathclose`: closing delimiter character class, with all spacing forced to zero.
    Close,
    /// `\mathrel`: relation character class.
    Rel,
    /// `\mathpunct`: punctuation character class.
    Punct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextToken {
    Accent(char),
    Letter(char),
    Style(HtmlTextStyle),
}

/// The delimiter pair that surrounds the result of an infix fraction-like command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfixDelim {
    /// Parentheses: `(` and `)` (`\choose`).
    Paren,
    /// Curly brackets: `{` and `}` (`\brace`).
    Brace,
    /// Square brackets: `[` and `]` (`\brack`).
    Brack,
}

/// For [`Token::ForceOpen`] and [`Token::ForceClose`]:
/// whether to force this token to be stretchy
/// (when combined with [`Token::Left`]/[`Token::Right`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceStretchy {
    /// Never apply `stretchy="true"`, and don't allow
    /// combining with `\left`/`\right`
    No,
    /// Allow combining with `\left`/`\right`,
    /// but don't actually use `stretchy="true"`.
    /// Used for the corner brackets
    Pretend,
    /// Allow combining with `\left`/`\right`,
    /// applying `stretchy="true"`
    Yes,
}

/// Disambiguates `\phantom`, `\hphantom`, and `\vphantom`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhantomKind {
    /// `\phantom`
    Full,
    /// `\hphantom`
    H,
    /// `\vphantom`
    V,
}

#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(Token<'_>, [usize; 3]);
#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(Result<Token<'_>, &'static i32>, [usize; 3]);

impl Token<'_> {
    /// Returns the character class of this token.
    pub(super) fn class(&self) -> Option<Class> {
        use Token::*;
        match self.unwrap_math_ref() {
            Relation(_) | ForceRelation(_) | XArrow(_) => Some(Class::Relation),
            Punctuation(_) | ForcePunctuation(_) => Some(Class::Punctuation),
            Open(_) | Left | SquareBracketOpen | ForceOpen(..) | Begin(_) | GroupBegin => {
                Some(Class::Open)
            }
            Close(_) | SquareBracketClose | ForceClose(..) | Right => Some(Class::Close),
            BinaryOp(_) | ForceBinaryOp(_) => Some(Class::BinaryOp),
            Op(_) | PseudoOperator(_) | PseudoOperatorLimits(_) | OperatorName { .. } => {
                Some(Class::Operator)
            }
            End(_) | NewLine | NewColumn | GroupEnd | Eoi => Some(Class::End),
            Inner(_) => Some(Class::Inner),
            Big(_, Some(paren_type)) => Some(match paren_type {
                ParenType::Left => Class::Open,
                ParenType::Right => Class::Close,
                ParenType::Middle => Class::Relation,
            }),
            MathClass(kind) => Some(match kind {
                MathClassKind::Ord => Class::Default,
                MathClassKind::Bin => Class::BinaryOp,
                MathClassKind::Open => Class::Open,
                MathClassKind::Close => Class::Close,
                MathClassKind::Rel => Class::Relation,
                MathClassKind::Punct => Class::Punctuation,
            }),
            CustomCmd(_, toks) => toks.iter().find_map(Token::class),
            Whitespace | Space(_) | Overlay(_) | TransformSwitch(_) | NoNumber | Tag
            | CustomSpace | Limits | NonBreakingSpace | Label | EqRef => None,
            Letter(_, _)
            | UprightLetter(_)
            | Digit(_)
            | Big(_, None)
            | Middle
            | Frac(_)
            | InfixGenFrac { .. }
            | Genfrac
            | Underscore
            | Circumflex
            | Prescript
            | Sideset
            | Binom(_)
            | Overset
            | Underset
            | OverUnderBrace(_, _)
            | Sqrt
            | Transform(_)
            | Ord(_)
            | Prime
            | Enclose(_)
            | Text(_)
            | Style(_)
            | Cramped
            | Color
            | Phantom(_)
            | CustomCmdArg(_)
            | TextMode(_)
            | MathOrTextMode(_, _)
            | UnknownCommand(_)
            | InternalStringLiteral(_)
            | Accent(_, _, _) => Some(Class::Default),
        }
    }

    /// If this token is `MathOrTextMode`, returns the inner token. Otherwise, returns `self`.
    #[inline]
    pub fn unwrap_math_ref(&self) -> &Self {
        if let Token::MathOrTextMode(tok, _) = self {
            tok
        } else {
            self
        }
    }
    #[inline]
    pub fn unwrap_math(self) -> Self {
        if let Token::MathOrTextMode(tok, _) = self {
            *tok
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Math,
    MathOrText,
}

#[derive(Clone, Copy, Debug, Default)]
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
#[derive(Clone, Copy, Debug)]
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
}

impl<'config> From<Token<'config>> for TokSpan<'config> {
    #[inline]
    fn from(token: Token<'config>) -> Self {
        TokSpan(token, Span::default())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoStaticStr)]
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
    pub fn matches(self, other: &Token) -> bool {
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
