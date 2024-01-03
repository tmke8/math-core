use std::fmt;

use bumpalo::{collections::String, Bump};

use crate::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum LatexError<'a> {
    UnexpectedToken {
        expected: Token<'a>,
        got: Token<'a>,
    },
    UnexpectedClose(Token<'a>),
    UnexpectedEOF,
    MissingParenthesis {
        location: Token<'a>,
        got: Token<'a>,
    },
    UnknownEnvironment(String<'a>),
    UnknownCommand(String<'a>),
    MismatchedEnvironment {
        expected: String<'a>,
        got: String<'a>,
    },
    InvalidCharacter {
        expected: &'static str,
        got: char,
    },
}

impl<'a> LatexError<'a> {
    /// Returns the error message as a string.
    ///
    /// This serves the same purpose as the `Display` implementation,
    /// but produces more compact WASM code.
    pub fn string(&self, alloc: &'a Bump) -> String<'a> {
        let s = String::new_in(alloc);
        match self {
            LatexError::UnexpectedToken { expected, got } => {
                s + "Expected token \""
                    + expected.as_ref()
                    + "\", but found token \""
                    + got.as_ref()
                    + "\"."
            }
            LatexError::UnexpectedClose(got) => {
                s + "Unexpected closing token: \"" + got.as_ref() + "\"."
            }
            LatexError::UnexpectedEOF => s + "Unexpected end of file.",
            LatexError::MissingParenthesis { location, got } => {
                s + "There must be a parenthesis after \""
                    + location.as_ref()
                    + "\", but not found. Instead, \""
                    + got.as_ref()
                    + "\" was found."
            }
            LatexError::UnknownEnvironment(environment) => {
                s + "Unknown environment \"" + environment + "\"."
            }
            LatexError::UnknownCommand(cmd) => s + "Unknown command \"\\" + cmd + "\".",
            LatexError::MismatchedEnvironment { expected, got } => {
                s + "Expected \"\\end{" + expected + "}\", but got \"\\end{" + got + "}\""
            }
            LatexError::InvalidCharacter { expected, got } => {
                // 4-byte buffer is enough for any UTF-8 character.
                let mut b = [0; 4];
                s + "Expected " + expected + ", but got \"" + got.encode_utf8(&mut b) + "\"."
            }
        }
    }
}

impl fmt::Display for LatexError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alloc = Bump::new();
        let output = self.string(&alloc);
        write!(f, "{}", output)
    }
}

impl std::error::Error for LatexError<'_> {}
