use std::fmt;

use crate::token::Token;

#[derive(Debug)]
pub enum LatexError<'a> {
    UnexpectedToken {
        expected: Token<'a>,
        got: Token<'a>,
    },
    UnclosedGroup(Token<'a>),
    UnexpectedClose(Token<'a>),
    UnexpectedEOF,
    MissingParenthesis {
        location: Token<'a>,
        got: Token<'a>,
    },
    UnknownEnvironment(&'a str),
    UnknownCommand(&'a str),
    MismatchedEnvironment {
        expected: &'a str,
        got: &'a str,
    },
    CannotBeUsedHere {
        got: Token<'a>,
        correct_place: &'static str,
    },
}

impl<'a> LatexError<'a> {
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
        }
    }
}

impl fmt::Display for LatexError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

impl std::error::Error for LatexError<'_> {}
