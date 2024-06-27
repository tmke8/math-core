use std::fmt;

use crate::token::Token;

#[derive(Debug)]
pub enum LatexError<'source> {
    UnexpectedToken {
        expected: Token<'source>,
        got: Token<'source>,
    },
    UnclosedGroup(Token<'source>),
    UnexpectedClose(Token<'source>),
    UnexpectedEOF,
    MissingParenthesis {
        location: Token<'source>,
        got: Token<'source>,
    },
    UnknownEnvironment(&'source str),
    UnknownCommand(&'source str),
    MismatchedEnvironment {
        expected: &'source str,
        got: &'source str,
    },
    CannotBeUsedHere {
        got: Token<'source>,
        correct_place: &'static str,
    },
    ExpectedText(&'static str),
}

impl LatexError<'_> {
    /// Returns the error message as a string.
    ///
    /// This serves the same purpose as the `Display` implementation,
    /// but produces more compact WASM code.
    pub fn string(&self) -> String {
        match self {
            LatexError::UnexpectedToken { expected, got } => {
                "Expected token \"".to_string()
                    + expected.as_ref()
                    + "\", but found token \""
                    + got.as_ref()
                    + "\"."
            }
            LatexError::UnclosedGroup(expected) => {
                "Expected token \"".to_string() + expected.as_ref() + "\", but not found."
            }
            LatexError::UnexpectedClose(got) => {
                "Unexpected closing token: \"".to_string() + got.as_ref() + "\"."
            }
            LatexError::UnexpectedEOF => "Unexpected end of file.".to_string(),
            LatexError::MissingParenthesis { location, got } => {
                "There must be a parenthesis after \"".to_string()
                    + location.as_ref()
                    + "\", but not found. Instead, \""
                    + got.as_ref()
                    + "\" was found."
            }
            LatexError::UnknownEnvironment(environment) => {
                "Unknown environment \"".to_string() + environment + "\"."
            }
            LatexError::UnknownCommand(cmd) => "Unknown command \"\\".to_string() + cmd + "\".",
            LatexError::MismatchedEnvironment { expected, got } => {
                "Expected \"\\end{".to_string() + expected + "}\", but got \"\\end{" + got + "}\""
            }
            LatexError::CannotBeUsedHere {
                got,
                correct_place: needs,
            } => "Got \"".to_string() + got.as_ref() + "\", which may only appear " + needs + ".",
            LatexError::ExpectedText(place) => "Expected text in ".to_string() + place + ".",
        }
    }
}

impl fmt::Display for LatexError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

impl std::error::Error for LatexError<'_> {}

pub trait ExpectOptim {
    type Inner;
    /// Optimized version of `Option::expect`.
    fn expect_optim(self, msg: &str) -> Self::Inner;
}

impl<T> ExpectOptim for Option<T> {
    type Inner = T;
    #[cfg(target_arch = "wasm32")]
    #[inline]
    fn expect_optim(self, _msg: &str) -> Self::Inner {
        // On WASM, panics are really expensive in terms of code size,
        // so we use an unchecked unwrap here.
        unsafe { self.unwrap_unchecked() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    fn expect_optim(self, msg: &str) -> Self::Inner {
        self.expect(msg)
    }
}

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
