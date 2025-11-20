use std::fmt::{self, Write};

use strum_macros::IntoStaticStr;

// use no_panic::no_panic;

use crate::MathDisplay;
use crate::html_utils::{escape_double_quoted_html_attribute, escape_html_content};

use super::environments::Env;
use super::token::Token;

/// Represents an error that occurred during LaTeX parsing or rendering.
#[derive(Debug, Clone, Copy)]
pub struct LatexError<'source>(pub usize, pub LatexErrKind<'source>);

#[derive(Debug, Clone, Copy)]
pub enum LatexErrKind<'source> {
    UnexpectedToken {
        expected: &'static Token<'static>,
        got: Token<'source>,
    },
    UnclosedGroup(Token<'source>),
    UnexpectedClose(Token<'source>),
    UnexpectedEOF,
    MissingParenthesis {
        location: &'static Token<'static>,
        got: Token<'source>,
    },
    DisallowedChar(char),
    UnknownEnvironment(&'source str),
    UnknownCommand(&'source str),
    UnknownColor(&'source str),
    MismatchedEnvironment {
        expected: Env,
        got: Env,
    },
    CannotBeUsedHere {
        got: Token<'source>,
        correct_place: Place,
    },
    ExpectedText(&'static str),
    ExpectedLength(&'source str),
    ExpectedColSpec(&'source str),
    ExpectedNumber(&'source str),
    RenderError,
    NotValidInTextMode(Token<'source>),
    InvalidMacroName(&'source str),
    InvalidParameterNumber,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
#[repr(u32)] // A different value here somehow increases code size on WASM enormously.
pub enum Place {
    #[strum(serialize = r"after \int, \sum, ...")]
    AfterBigOp,
    #[strum(serialize = r"before supported operators")]
    BeforeSomeOps,
    #[strum(serialize = r"after an identifier or operator")]
    AfterOpOrIdent,
    #[strum(serialize = r"in a table-like environment")]
    TableEnv,
    #[strum(serialize = r"in a numbered equation environment")]
    NumberedEnv,
}

impl LatexErrKind<'_> {
    /// Returns the error message as a string.
    ///
    /// This serves the same purpose as the `Display` implementation,
    /// but produces more compact WASM code.
    pub fn string(&self) -> String {
        match self {
            LatexErrKind::UnexpectedToken { expected, got } => {
                "Expected token \"".to_string()
                    + <&str>::from(*expected)
                    + "\", but found token \""
                    + <&str>::from(got)
                    + "\"."
            }
            LatexErrKind::UnclosedGroup(expected) => {
                "Expected token \"".to_string() + <&str>::from(expected) + "\", but not found."
            }
            LatexErrKind::UnexpectedClose(got) => {
                "Unexpected closing token: \"".to_string() + <&str>::from(got) + "\"."
            }
            LatexErrKind::UnexpectedEOF => "Unexpected end of file.".to_string(),
            LatexErrKind::MissingParenthesis { location, got } => {
                "There must be a parenthesis after \"".to_string()
                    + <&str>::from(*location)
                    + "\", but not found. Instead, \""
                    + <&str>::from(got)
                    + "\" was found."
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
                    + <&str>::from(expected)
                    + "}\", but got \"\\end{"
                    + <&str>::from(got)
                    + "}\"."
            }
            LatexErrKind::CannotBeUsedHere { got, correct_place } => {
                "Got \"".to_string()
                    + <&str>::from(got)
                    + "\", which may only appear "
                    + <&str>::from(correct_place)
                    + "."
            }
            LatexErrKind::ExpectedText(place) => "Expected text in ".to_string() + place + ".",
            LatexErrKind::ExpectedLength(got) => {
                "Expected length with units, got \"".to_string() + got + "\"."
            }
            LatexErrKind::ExpectedNumber(got) => "Expected a number, got \"".to_string() + got + "\".",
            LatexErrKind::ExpectedColSpec(got) => {
                "Expected column specification, got \"".to_string() + got + "\"."
            }
            LatexErrKind::RenderError => "Render error".to_string(),
            LatexErrKind::NotValidInTextMode(got) => {
                "Got \"".to_string() + <&str>::from(got) + "\", which is not valid in text mode."
            }
            LatexErrKind::InvalidMacroName(name) => {
                "Invalid macro name: \"\\".to_string() + name + "\"."
            }
            LatexErrKind::InvalidParameterNumber => {
                "Invalid parameter number. Must be 1-9.".to_string()
            }
            LatexErrKind::Internal => {
                "Internal parser error. Please report this bug at https://github.com/tmke8/math-core/issues".to_string()
            },
        }
    }
}

impl LatexError<'_> {
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
            tag, css_class, self.0
        );
        escape_double_quoted_html_attribute(&mut output, &self.1.string());
        output.push_str(r#""><code>"#);
        escape_html_content(&mut output, latex);
        let _ = write!(output, "</code></{tag}>");
        output
    }
}

impl fmt::Display for LatexError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.0, self.1.string())
    }
}

impl std::error::Error for LatexError<'_> {}

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

/* fn itoa(val: u64, buf: &mut [u8; 20]) -> &str {
    let start = if val == 0 {
        buf[19] = b'0';
        19
    } else {
        let mut val = val;
        let mut i = 20;

        // The `i > 0` check is technically redundant but it allows the compiler to
        // prove that `buf` is always validly indexed.
        while val != 0 && i > 0 {
            i -= 1;
            buf[i] = (val % 10) as u8 + b'0';
            val /= 10;
        }
        i
    };

    // This is safe because we know the buffer contains valid ASCII.
    // This unsafe block wouldn't be necessary if the `ascii_char` feature were stable.
    unsafe { std::str::from_utf8_unchecked(&buf[start..]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itoa_test() {
        let mut buf = [0u8; 20];
        assert_eq!(itoa(0, &mut buf), "0");
        assert_eq!(itoa(1, &mut buf), "1");
        assert_eq!(itoa(10, &mut buf), "10");
        assert_eq!(itoa(1234567890, &mut buf), "1234567890");
        assert_eq!(itoa(u64::MAX, &mut buf), "18446744073709551615");
    }
} */
