use std::fmt::{self, Write};
use std::ops::Range;

use strum_macros::IntoStaticStr;

use crate::MathDisplay;
use crate::environments::Env;
use crate::html_utils::{escape_double_quoted_html_attribute, escape_html_content};
use crate::token::EndToken;

/// Represents an error that occurred during LaTeX parsing.
#[derive(Debug, Clone)]
pub struct LatexError(pub Range<usize>, pub(crate) LatexErrKind);

#[derive(Debug, Clone)]
pub(crate) enum LatexErrKind {
    UnclosedGroup(EndToken),
    UnmatchedClose(EndToken),
    ExpectedArgumentGotClose,
    ExpectedArgumentGotEOF,
    ExpectedDelimiter(DelimiterModifier),
    DisallowedChar(char),
    UnknownEnvironment(Box<str>),
    UnknownCommand(Box<str>),
    UnknownColor(Box<str>),
    MismatchedEnvironment {
        expected: Env,
        got: Env,
    },
    CannotBeUsedHere {
        got: LimitedUsabilityToken,
        correct_place: Place,
    },
    ExpectedRelation,
    BoundFollowedByBound,
    DuplicateSubOrSup,
    ExpectedText(&'static str),
    ExpectedLength(Box<str>),
    ExpectedColSpec(Box<str>),
    ExpectedNumber(Box<str>),
    NotValidInTextMode,
    InvalidMacroName(String),
    InvalidParameterNumber,
    MacroParameterOutsideCustomCommand,
    ExpectedParamNumberGotEOF,
    HardLimitExceeded,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
pub enum DelimiterModifier {
    #[strum(serialize = r"\left")]
    Left,
    #[strum(serialize = r"\right")]
    Right,
    #[strum(serialize = r"\middle")]
    Middle,
    #[strum(serialize = r"\big, \Big, ...")]
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[repr(u32)] // A different value here somehow increases code size on WASM enormously.
pub enum Place {
    #[strum(serialize = r"after \int, \sum, ...")]
    AfterBigOp,
    #[strum(serialize = r"in a table-like environment")]
    TableEnv,
    #[strum(serialize = r"in a numbered equation environment")]
    NumberedEnv,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
pub enum LimitedUsabilityToken {
    #[strum(serialize = "&")]
    Ampersand,
    #[strum(serialize = r"\tag")]
    Tag,
    #[strum(serialize = r"\limits")]
    Limits,
}

impl LatexErrKind {
    /// Returns the error message as a string.
    fn write_msg(&self, s: &mut String) -> std::fmt::Result {
        match self {
            LatexErrKind::UnclosedGroup(expected) => {
                write!(
                    s,
                    "Expected token \"{}\", but not found.",
                    <&str>::from(expected)
                )?;
            }
            LatexErrKind::UnmatchedClose(got) => {
                write!(s, "Unmatched closing token: \"{}\".", <&str>::from(got))?;
            }
            LatexErrKind::ExpectedArgumentGotClose => {
                write!(
                    s,
                    r"Expected argument but got closing token (`}}`, `\end`, `\right`)."
                )?;
            }
            LatexErrKind::ExpectedArgumentGotEOF => {
                write!(s, "Expected argument but reached end of input.")?;
            }
            LatexErrKind::ExpectedDelimiter(location) => {
                write!(
                    s,
                    "There must be a parenthesis after \"{}\", but not found.",
                    <&str>::from(*location)
                )?;
            }
            LatexErrKind::DisallowedChar(got) => {
                write!(s, "Disallowed character in text group: '{}'.", got)?;
            }
            LatexErrKind::UnknownEnvironment(environment) => {
                write!(s, "Unknown environment \"{}\".", environment)?;
            }
            LatexErrKind::UnknownCommand(cmd) => {
                write!(s, "Unknown command \"\\{}\".", cmd)?;
            }
            LatexErrKind::UnknownColor(color) => {
                write!(s, "Unknown color \"{}\".", color)?;
            }
            LatexErrKind::MismatchedEnvironment { expected, got } => {
                write!(
                    s,
                    "Expected \"\\end{{{}}}\", but got \"\\end{{{}}}\".",
                    expected.as_str(),
                    got.as_str()
                )?;
            }
            LatexErrKind::CannotBeUsedHere { got, correct_place } => {
                write!(
                    s,
                    "Got \"{}\", which may only appear {}.",
                    <&str>::from(got),
                    <&str>::from(correct_place)
                )?;
            }
            LatexErrKind::ExpectedRelation => {
                write!(s, "Expected a relation after \\not.")?;
            }
            LatexErrKind::BoundFollowedByBound => {
                write!(s, "'^' or '_' directly followed by '^', '_' or prime.")?;
            }
            LatexErrKind::DuplicateSubOrSup => {
                write!(s, "Duplicate subscript or superscript.")?;
            }
            LatexErrKind::ExpectedText(place) => {
                write!(s, "Expected text in {}.", place)?;
            }
            LatexErrKind::ExpectedLength(got) => {
                write!(s, "Expected length with units, got \"{}\".", got)?;
            }
            LatexErrKind::ExpectedNumber(got) => {
                write!(s, "Expected a number, got \"{}\".", got)?;
            }
            LatexErrKind::ExpectedColSpec(got) => {
                write!(s, "Expected column specification, got \"{}\".", got)?;
            }
            LatexErrKind::NotValidInTextMode => {
                write!(s, "Not valid in text mode.")?;
            }
            LatexErrKind::InvalidMacroName(name) => {
                write!(s, "Invalid macro name: \"\\{}\".", name)?;
            }
            LatexErrKind::InvalidParameterNumber => {
                write!(s, "Invalid parameter number. Must be 1-9.")?;
            }
            LatexErrKind::MacroParameterOutsideCustomCommand => {
                write!(
                    s,
                    "Macro parameter found outside of custom command definition."
                )?;
            }
            LatexErrKind::ExpectedParamNumberGotEOF => {
                write!(
                    s,
                    "Expected parameter number after '#', but got end of input."
                )?;
            }
            LatexErrKind::HardLimitExceeded => {
                write!(s, "Hard limit exceeded. Please simplify your equation.")?;
            }
            LatexErrKind::Internal => {
                write!(
                    s,
                    "Internal parser error. Please report this bug at https://github.com/tmke8/math-core/issues"
                )?;
            }
        }
        Ok(())
    }
}

impl LatexError {
    /// Format a LaTeX error as an HTML snippet.
    ///
    /// # Arguments
    /// - `latex`: The original LaTeX input that caused the error.
    /// - `display`: The display mode of the equation (inline or block).
    /// - `css_class`: An optional CSS class to apply to the error element. If `None`,
    ///   defaults to `"math-core-error"`.
    pub fn to_html(&self, latex: &str, display: MathDisplay, css_class: Option<&str>) -> String {
        let mut output = String::new();
        let tag = if matches!(display, MathDisplay::Block) {
            "p"
        } else {
            "span"
        };
        let css_class = css_class.unwrap_or("math-core-error");
        let _ = write!(
            output,
            r#"<{} class="{}" title="{}: "#,
            tag, css_class, self.0.start
        );
        escape_double_quoted_html_attribute(&mut output, &self.error_message());
        output.push_str(r#""><code>"#);
        escape_html_content(&mut output, latex);
        let _ = write!(output, "</code></{tag}>");
        output
    }

    pub fn error_message(&self) -> String {
        let mut s = String::new();
        let _ = self.1.write_msg(&mut s);
        s
    }
}

#[cfg(feature = "ariadne")]
impl LatexError {
    /// Convert this error into an [`ariadne::Report`] for pretty-printing.
    pub fn to_report<'name>(
        &self,
        source_name: &'name str,
        with_color: bool,
    ) -> ariadne::Report<'static, (&'name str, Range<usize>)> {
        use ariadne::{Label, Report, ReportKind};

        let label_msg = match &self.1 {
            LatexErrKind::UnclosedGroup(expected) => {
                format!(
                    "expected \"{}\" to close this group",
                    <&str>::from(expected)
                )
            }
            LatexErrKind::UnmatchedClose(got) => {
                format!("unmatched \"{}\"", <&str>::from(got))
            }
            LatexErrKind::ExpectedArgumentGotClose => "expected an argument here".into(),
            LatexErrKind::ExpectedArgumentGotEOF => "expected an argument here".into(),
            LatexErrKind::ExpectedDelimiter(modifier) => {
                format!("expected a delimiter after \"{}\"", <&str>::from(*modifier))
            }
            LatexErrKind::DisallowedChar(_) => "disallowed character".into(),
            LatexErrKind::UnknownEnvironment(_) => "unknown environment".into(),
            LatexErrKind::UnknownCommand(_) => "unknown command".into(),
            LatexErrKind::UnknownColor(_) => "unknown color".into(),
            LatexErrKind::MismatchedEnvironment { expected, .. } => {
                format!("expected \"\\end{{{}}}\" here", expected.as_str(),)
            }
            LatexErrKind::CannotBeUsedHere { correct_place, .. } => {
                format!("may only appear {}", <&str>::from(correct_place))
            }
            LatexErrKind::ExpectedRelation => "expected a relation".into(),
            LatexErrKind::BoundFollowedByBound => "unexpected bound".into(),
            LatexErrKind::DuplicateSubOrSup => "duplicate".into(),
            LatexErrKind::ExpectedText(place) => format!("expected text in {place}"),
            LatexErrKind::ExpectedLength(_) => "expected length here".into(),
            LatexErrKind::ExpectedNumber(_) => "expected a number here".into(),
            LatexErrKind::ExpectedColSpec(_) => "expected a column spec here".into(),
            LatexErrKind::NotValidInTextMode => "not valid in text mode".into(),
            LatexErrKind::InvalidMacroName(_) => "invalid name here".into(),
            LatexErrKind::InvalidParameterNumber => "must be 1-9".into(),
            LatexErrKind::MacroParameterOutsideCustomCommand => "unexpected macro parameter".into(),
            LatexErrKind::ExpectedParamNumberGotEOF => "expected parameter number".into(),
            LatexErrKind::HardLimitExceeded => "limit exceeded".into(),
            LatexErrKind::Internal => "internal error".into(),
        };

        let mut config = ariadne::Config::default().with_index_type(ariadne::IndexType::Byte);
        if !with_color {
            config = config.with_color(false);
        }
        Report::build(ReportKind::Error, (source_name, self.0.start..self.0.start))
            .with_config(config)
            .with_message(self.error_message())
            .with_label(Label::new((source_name, self.0.clone())).with_message(label_msg))
            .finish()
    }
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.0.start, self.error_message())
    }
}

impl std::error::Error for LatexError {}

pub trait GetUnwrap {
    /// `str::get` with `Option::unwrap`.
    fn get_unwrap(&self, range: std::ops::Range<usize>) -> &str;
}

impl GetUnwrap for str {
    #[cfg(target_arch = "wasm32")]
    #[inline]
    fn get_unwrap(&self, range: std::ops::Range<usize>) -> &str {
        // On WASM, panics are really expensive in terms of code size,
        // so we use an unchecked get here.
        unsafe { self.get_unchecked(range) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    fn get_unwrap(&self, range: std::ops::Range<usize>) -> &str {
        self.get(range).expect("valid range")
    }
}
