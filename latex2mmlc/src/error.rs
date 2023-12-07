use std::fmt;
use super::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum LatexError {
    UnexpectedToken {
        expected: Token, got: Token,
    },
    UnexpectedClose(Token),
    UnexpectedEOF,
    MissingParenthesis {
        location: Token, got: Token,
    },
    UnknownEnvironment(String),
    UnknownCommand(String),
    MismatchedEnvironment{ expected: String, got: String },
    InvalidCharacter {
        expected: &'static str,
        got: char,
    }
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatexError::UnexpectedToken{expected, got} => write!(f, 
                "The token \"{:?}\" is expected, but the token \"{:?}\" was found.", 
                expected, got
            ),
            LatexError::UnexpectedClose(got) => write!(f,
                "Unexpected closing token: \"{:?}\"", got
            ),
            LatexError::UnexpectedEOF => write!(f,
                "Unexpected end of file"
            ),
            LatexError::MissingParenthesis{location, got} => write!(f, 
                "There must be a parenthesis after \"{:?}\", but not found. Insted, \"{:?}\" was found.",
                location, got
            ),
            LatexError::UnknownEnvironment(environment) => write!(f,
                "Unknown environment \"{}\"", environment
            ),
            LatexError::UnknownCommand(cmd) => write!(f,
                "Unknown command \"\\{}\"", cmd
            ),
            LatexError::MismatchedEnvironment { expected, got } => write!(f,
                "Expected \"\\end{{{}}}\", but got \"\\end{{{}}}\"", expected, got
            ),
            LatexError::InvalidCharacter { expected, got } => write!(f,
                "Expected {}, but got \"{}\"", expected, got
            ),
                    
        }
    }
}

impl std::error::Error for LatexError {}
