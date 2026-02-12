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
    ///
    /// This serves the same purpose as the `Display` implementation,
    /// but produces more compact WASM code.
    pub fn string(&self) -> String {
        match self {
            LatexErrKind::UnclosedGroup(expected) => {
                "Expected token \"".to_string() + <&str>::from(expected) + "\", but not found."
            }
            LatexErrKind::UnmatchedClose(got) => {
                "Unmatched closing token: \"".to_string() + <&str>::from(got) + "\"."
            }
            LatexErrKind::ExpectedArgumentGotClose => {
                r"Expected argument but got closing token (`}`, `\end`, `\right`).".to_string()
            }
            LatexErrKind::ExpectedArgumentGotEOF => "Expected argument but reached end of input.".to_string(),
            LatexErrKind::ExpectedDelimiter(location) => {
                "There must be a parenthesis after \"".to_string()
                    + <&str>::from(*location)
                    + "\", but not found."
            }
            LatexErrKind::DisallowedChar(got) => {
                let mut text = "Disallowed character in text group: '".to_string();
                text.push(*got);
                text += "'.";
                text
            }
            LatexErrKind::UnknownEnvironment(environment) => {
                "Unknown environment \"".to_string() + environment + "\"."
            }
            LatexErrKind::UnknownCommand(cmd) => "Unknown command \"\\".to_string() + cmd + "\".",
            LatexErrKind::UnknownColor(color) => "Unknown color \"".to_string() + color + "\".",
            LatexErrKind::MismatchedEnvironment { expected, got } => {
                "Expected \"\\end{".to_string()
                    + expected.as_str()
                    + "}\", but got \"\\end{"
                    + got.as_str()
                    + "}\"."
            }
            LatexErrKind::CannotBeUsedHere { got, correct_place } => {
                "Got \"".to_string()
                    + <&str>::from(got)
                    + "\", which may only appear "
                    + <&str>::from(correct_place)
                    + "."
            }
            LatexErrKind::ExpectedRelation => {
                "Expected a relation after \\not.".to_string()
            }
            LatexErrKind::BoundFollowedByBound => {
                "'^' or '_' directly followed by '^', '_' or prime.".to_string()
            }
            LatexErrKind::DuplicateSubOrSup => {
                "Duplicate subscript or superscript.".to_string()
            }
            LatexErrKind::ExpectedText(place) => "Expected text in ".to_string() + place + ".",
            LatexErrKind::ExpectedLength(got) => {
                "Expected length with units, got \"".to_string() + got + "\"."
            }
            LatexErrKind::ExpectedNumber(got) => "Expected a number, got \"".to_string() + got + "\".",
            LatexErrKind::ExpectedColSpec(got) => {
                "Expected column specification, got \"".to_string() + got + "\"."
            }
            LatexErrKind::NotValidInTextMode => {
                "Not valid in text mode.".to_string()
            }
            LatexErrKind::InvalidMacroName(name) => {
                "Invalid macro name: \"\\".to_string() + name + "\"."
            }
            LatexErrKind::InvalidParameterNumber => {
                "Invalid parameter number. Must be 1-9.".to_string()
            }
            LatexErrKind::MacroParameterOutsideCustomCommand => {
                "Macro parameter found outside of custom command definition.".to_string()
            }
            LatexErrKind::ExpectedParamNumberGotEOF => {
                "Expected parameter number after '#', but got end of input.".to_string()
            }
            LatexErrKind::HardLimitExceeded => {
                "Hard limit exceeded. Please simplify your equation.".to_string()
            }
            LatexErrKind::Internal => {
                "Internal parser error. Please report this bug at https://github.com/tmke8/math-core/issues".to_string()
            },
        }
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
        escape_double_quoted_html_attribute(&mut output, &self.1.string());
        output.push_str(r#""><code>"#);
        escape_html_content(&mut output, latex);
        let _ = write!(output, "</code></{tag}>");
        output
    }

    pub fn error_message(&self) -> String {
        self.1.string()
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
            .with_message(self.1.string())
            .with_label(Label::new((source_name, self.0.clone())).with_message(label_msg))
            .finish()
    }
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.0.start, self.1.string())
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
