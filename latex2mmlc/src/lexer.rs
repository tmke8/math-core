//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!

use crate::ops::Op;
use crate::{ops, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'a> {
    input: std::str::CharIndices<'a>,
    cur: char,
    offset: usize,
    input_string: &'a str,
    input_length: usize,
}

impl<'a> Lexer<'a> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.char_indices(),
            cur: '\u{0}',
            offset: 0,
            input_string: input,
            input_length: input.len(),
        };
        lexer.read_char();
        lexer
    }

    /// One character progresses.
    fn read_char(&mut self) {
        (self.offset, self.cur) = self.input.next().unwrap_or((self.input_length, '\u{0}'));
    }

    /// Skip blank characters.
    fn skip_whitespace(&mut self) {
        while self.cur.is_ascii_whitespace() {
            self.read_char();
        }
    }

    /// Read one command to a token.
    #[inline]
    fn read_command(&mut self) -> &'a str {
        // Always read at least one character.
        self.read_char();
        let start = self.offset;

        if !self.cur.is_ascii_alphabetic() {
            self.read_char();
            // SAFETY: we got `start` and `offset` from `CharIndices`, so they are valid bounds.
            return unsafe { self.input_string.get_unchecked(start..self.offset) };
        }

        // Read in all ASCII characters.
        self.read_char();
        while self.cur.is_ascii_alphabetic() {
            self.read_char();
        }
        // SAFETY: we got `start` and `offset` from `CharIndices`, so they are valid bounds.
        unsafe { self.input_string.get_unchecked(start..self.offset) }
    }

    /// Read one number into a token.
    fn read_number(&mut self) -> (&'a str, Op) {
        let start = self.offset;
        while self.cur.is_ascii_digit() || matches!(self.cur, '.' | ',') {
            // Before we accept the current character, we need to check the next one.
            let candidate = self.cur;
            let end = self.offset;
            self.read_char();
            if matches!(candidate, '.' | ',') && !self.cur.is_ascii_digit() {
                // If the candidate is punctuation and the next character is not a digit,
                // we don't want to include the punctuation.
                // But we do need to return the punctuation as an operator.
                let number = unsafe { self.input_string.get_unchecked(start..end) };
                let op = match candidate {
                    '.' => ops::FULL_STOP,
                    ',' => ops::COMMA,
                    _ => unreachable!(),
                };
                return (number, op);
            }
        }
        let number = unsafe { self.input_string.get_unchecked(start..self.offset) };
        (number, ops::NULL)
    }

    /// Read text until the next `}`.
    pub(crate) fn read_text_content(&mut self) -> Option<&'a str> {
        let start = self.offset;
        while self.cur != '}' {
            if self.cur == '\u{0}' {
                return None;
            }
            self.read_char();
        }
        let end = self.offset;
        self.read_char(); // Discard the closing brace.
        unsafe { Some(self.input_string.get_unchecked(start..end)) }
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> Token<'a> {
        if wants_digit && self.cur.is_ascii_digit() {
            let start = self.offset;
            self.read_char();
            let num = unsafe { self.input_string.get_unchecked(start..self.offset) };
            return Token::Number(num, ops::NULL);
        }
        self.skip_whitespace();

        let token = match self.cur {
            '=' => Token::Operator(ops::EQUALS_SIGN),
            ';' => Token::Operator(ops::SEMICOLON),
            ',' => Token::Operator(ops::COMMA),
            '.' => Token::Operator(ops::FULL_STOP),
            '\'' => Token::Operator(ops::APOSTROPHE),
            '(' => Token::Paren(ops::LEFT_PARENTHESIS),
            ')' => Token::Paren(ops::RIGHT_PARENTHESIS),
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::Paren(ops::LEFT_SQUARE_BRACKET),
            ']' => Token::Paren(ops::RIGHT_SQUARE_BRACKET),
            '|' => Token::Paren(ops::VERTICAL_LINE),
            '+' => Token::Operator(ops::PLUS_SIGN),
            '-' => Token::Operator(ops::MINUS_SIGN),
            '*' => Token::Operator(ops::ASTERISK),
            '/' => Token::Operator(ops::SOLIDUS),
            '!' => Token::Operator(ops::EXCLAMATION_MARK),
            '<' => Token::OpLessThan,
            '>' => Token::OpGreaterThan,
            '_' => Token::Underscore,
            '^' => Token::Circumflex,
            '&' => Token::Ampersand,
            '~' => Token::NonBreakingSpace,
            '\u{0}' => Token::EOF,
            ':' => Token::Colon,
            ' ' => Token::Letter('\u{A0}'),
            '\\' => {
                return Token::from_command(self.read_command());
            }
            c => {
                if c.is_ascii_digit() {
                    let (num, op) = self.read_number();
                    return Token::Number(num, op);
                } else if c.is_ascii_alphabetic() {
                    Token::Letter(c)
                } else {
                    Token::NormalLetter(c)
                }
            }
        };
        self.read_char();
        token
    }
}

#[cfg(test)]
mod tests {
    use super::super::token::Token;
    use super::*;

    #[test]
    fn lexer_test() {
        let problems = vec![
            (r"3", vec![Token::Number("3", ops::NULL)]),
            (r"3.14", vec![Token::Number("3.14", ops::NULL)]),
            (r"3.14.", vec![Token::Number("3.14", ops::FULL_STOP)]),
            (
                r"3..14",
                vec![
                    Token::Number("3", ops::FULL_STOP),
                    Token::Operator(ops::FULL_STOP),
                    Token::Number("14", ops::NULL),
                ],
            ),
            (r"x", vec![Token::Letter('x')]),
            (r"\pi", vec![Token::Letter('π')]),
            (
                r"x = 3.14",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::EQUALS_SIGN),
                    Token::Number("3.14", ops::NULL),
                ],
            ),
            (r"\alpha\beta", vec![Token::Letter('α'), Token::Letter('β')]),
            (
                r"x+y",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::PLUS_SIGN),
                    Token::Letter('y'),
                ],
            ),
            (
                r"\ 1",
                vec![Token::Space("1"), Token::Number("1", ops::NULL)],
            ),
        ];

        for (problem, answer) in problems.iter() {
            let mut lexer = Lexer::new(problem);
            for answer in answer.iter() {
                assert_eq!(&lexer.next_token(false), answer);
            }
        }
    }
}
