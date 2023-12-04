use std::fmt;
use super::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum LatexError {
    UnexpectedToken {
        expected: Token, got: Token,
    },
    UnexpectedClose(Token),
    MissingParenthesis {
        location: Token, got: Token,
    },
    UnknownEnvironment(String),
    UnknownCommand(String),
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatexError::UnexpectedToken{expected, got} => write!(f, 
                "The token \"{:?}\" is expected, but the token \"{:?}\" was found.", 
                expected, got
            ),
            LatexError::UnexpectedClose(got) => write!(f,
                "Got an unexpected closing token: \"{:?}\"", got
            ),
            LatexError::MissingParenthesis{location, got} => write!(f, 
                "There must be a parenthesis after \"{:?}\", but not found. Insted, \"{:?}\" was found.",
                location, got
            ),
            LatexError::UnknownEnvironment(environment) => write!(f,
                "An unknown environment \"{}\"", environment
            ),
            LatexError::UnknownCommand(cmd) => write!(f,
                "An unknown command \"\\{}\"", cmd
            ),
        }
    }
}

impl std::error::Error for LatexError {}
