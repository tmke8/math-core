use super::token::Token;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum LatexError {
    UnexpectedToken { expected: Token, got: Token },
    UnexpectedClose(Token),
    UnexpectedEOF,
    MissingParenthesis { location: Token, got: Token },
    UnknownEnvironment(String),
    UnknownCommand(String),
    MismatchedEnvironment { expected: String, got: String },
    InvalidCharacter { expected: &'static str, got: char },
}

impl LatexError {
    /// Returns the error message as a string.
    ///
    /// This serves the same purpose as the `Display` implementation,
    /// but produces more compact WASM code.
    pub fn string(&self) -> String {
        let mut s = String::new();
        match self {
            LatexError::UnexpectedToken { expected, got } => {
                s.push_str("Expected token \"");
                s.push_str(expected.as_ref());
                s.push_str("\", but found token \"");
                s.push_str(got.as_ref());
                s.push_str("\".");
            }
            LatexError::UnexpectedClose(got) => {
                s.push_str("Unexpected closing token: \"");
                s.push_str(got.as_ref());
                s.push('"');
            }
            LatexError::UnexpectedEOF => s.push_str("Unexpected end of file"),
            LatexError::MissingParenthesis { location, got } => {
                s.push_str("There must be a parenthesis after \"");
                s.push_str(location.as_ref());
                s.push_str("\", but not found. Instead, \"");
                s.push_str(got.as_ref());
                s.push_str("\" was found.");
            }
            LatexError::UnknownEnvironment(environment) => {
                s.push_str("Unknown environment \"");
                s.push_str(environment);
                s.push('"');
            }
            LatexError::UnknownCommand(cmd) => {
                s.push_str("Unknown command \"\\");
                s.push_str(cmd);
                s.push('"');
            }
            LatexError::MismatchedEnvironment { expected, got } => {
                s.push_str("Expected \"\\end{");
                s.push_str(expected);
                s.push_str("}\", but got \"\\end{");
                s.push_str(got);
                s.push_str("}\"");
            }
            LatexError::InvalidCharacter { expected, got } => {
                s.push_str("Expected ");
                s.push_str(expected);
                s.push_str(", but got \"");
                s.push(*got);
                s.push('"');
            }
        }
        s
    }
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

impl std::error::Error for LatexError {}
