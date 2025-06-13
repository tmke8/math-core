use std::fmt;

use strum_macros::IntoStaticStr;

// use no_panic::no_panic;

use super::token::Token;

/// Represents an error that occurred during LaTeX parsing or rendering.
#[derive(Debug)]
pub struct LatexError<'source>(pub usize, pub LatexErrKind<'source>);

#[derive(Debug)]
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
    DisallowedChars,
    UnknownEnvironment(&'source str),
    UnknownCommand(&'source str),
    UnknownColor(&'source str),
    MismatchedEnvironment {
        expected: &'source str,
        got: &'source str,
    },
    CannotBeUsedHere {
        got: Token<'source>,
        correct_place: Place,
    },
    ExpectedText(&'static str),
    ExpectedLength(&'source str),
    ExpectedColSpec(&'source str),
    RenderError,
    NotValidInTextMode(Token<'source>),
    InvalidMacroName(&'source str),
}

#[derive(Debug, IntoStaticStr)]
#[repr(u32)] // A different value here somehow increases code size on WASM enormously.
pub enum Place {
    #[strum(serialize = r"after \int, \sum, ...")]
    AfterBigOp,
    #[strum(serialize = r"before supported operators")]
    BeforeSomeOps,
    #[strum(serialize = r"after an identifier or operator")]
    AfterOpOrIdent,
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
            LatexErrKind::DisallowedChars => "Disallowed characters in text group.".to_string(),
            LatexErrKind::UnknownEnvironment(environment) => {
                "Unknown environment \"".to_string() + environment + "\"."
            }
            LatexErrKind::UnknownCommand(cmd) => "Unknown command \"\\".to_string() + cmd + "\".",
            LatexErrKind::UnknownColor(color) => "Unknown color \"".to_string() + color + "\".",
            LatexErrKind::MismatchedEnvironment { expected, got } => {
                "Expected \"\\end{".to_string() + expected + "}\", but got \"\\end{" + got + "}\"."
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
        }
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
