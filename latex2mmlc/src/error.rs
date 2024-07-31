use std::fmt;

use strum_macros::AsRefStr;

// use no_panic::no_panic;

use crate::token::{TokLoc, Token};

#[derive(Debug)]
pub enum LatexError<'source> {
    UnexpectedToken {
        expected: &'static Token<'static>,
        got: TokLoc<'source>,
    },
    UnclosedGroup(Token<'source>),
    UnexpectedClose(TokLoc<'source>),
    UnexpectedEOF,
    MissingParenthesis {
        location: &'static Token<'static>,
        got: TokLoc<'source>,
    },
    UnknownEnvironment(&'source str),
    UnknownCommand(&'source str),
    MismatchedEnvironment {
        expected: &'source str,
        got: &'source str,
        loc: usize,
    },
    CannotBeUsedHere {
        got: TokLoc<'source>,
        correct_place: Place,
    },
    ExpectedText(&'static str),
}

#[derive(Debug, AsRefStr)]
#[repr(u32)] // A different value here somehow increases code size on WASM enormously.
pub enum Place {
    #[strum(serialize = r"after \int, \sum, ...")]
    AfterBigOp,
    #[strum(serialize = r"before supported operators")]
    BeforeSomeOps,
    #[strum(serialize = r"after an identifier or operator")]
    AfterOpOrIdent,
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
                    + got.token().as_ref()
                    + "\" at location "
                    + itoa(got.location() as u64)
                    + "."
            }
            LatexError::UnclosedGroup(expected) => {
                "Expected token \"".to_string() + expected.as_ref() + "\", but not found."
            }
            LatexError::UnexpectedClose(got) => {
                "Unexpected closing token: \"".to_string()
                    + got.token().as_ref()
                    + "\" at location "
                    + itoa(got.location() as u64)
                    + "."
            }
            LatexError::UnexpectedEOF => "Unexpected end of file.".to_string(),
            LatexError::MissingParenthesis { location, got } => {
                "There must be a parenthesis after \"".to_string()
                    + location.as_ref()
                    + "\", but not found. Instead, \""
                    + got.token().as_ref()
                    + "\" was found at location "
                    + itoa(got.location() as u64)
                    + "."
            }
            LatexError::UnknownEnvironment(environment) => {
                "Unknown environment \"".to_string() + environment + "\"."
            }
            LatexError::UnknownCommand(cmd) => "Unknown command \"\\".to_string() + cmd + "\".",
            LatexError::MismatchedEnvironment { expected, got, loc } => {
                "Expected \"\\end{".to_string()
                    + expected
                    + "}\", but got \"\\end{"
                    + got
                    + "}\" at location "
                    + itoa(*loc as u64)
                    + "."
            }
            LatexError::CannotBeUsedHere { got, correct_place } => {
                "Got \"".to_string()
                    + got.token().as_ref()
                    + "\" at location "
                    + itoa(got.location() as u64)
                    + ", which may only appear "
                    + correct_place.as_ref()
                    + "."
            }
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

// fn itoa(val: u64) -> &'static str {
//     ""
// }

static mut ITOA_BUF: [u8; 20] = [0; 20];

// #[no_panic]
fn itoa(val: u64) -> &'static str {
    if val == 0 {
        return "0";
    }
    let mut val = val;
    let base = 10;
    let digits = b"0123456789";
    let mut i = 20;

    while val != 0 && i > 0 {
        i -= 1;
        // let digit = unsafe { digits.get_unchecked((val % base) as usize) };
        let digit = digits[(val % base) as usize];
        unsafe { ITOA_BUF[i] = digit };
        val /= base;
    }

    let slice = unsafe { &ITOA_BUF[i..] };
    // This unsafe block wouldn't be necessary if the `ascii_char` feature were stable.
    unsafe { std::str::from_utf8_unchecked(slice) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itoa_test() {
        assert_eq!(itoa(0), "0");
        assert_eq!(itoa(1), "1");
        assert_eq!(itoa(10), "10");
        assert_eq!(itoa(1234567890), "1234567890");
        assert_eq!(itoa(u64::MAX), "18446744073709551615");
    }
}
