//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!

use std::iter::Peekable;
use std::str::CharIndices;

use crate::commands::get_command;
use crate::{ops, ops::Op, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'a> {
    input: Peekable<CharIndices<'a>>,
    input_string: &'a str,
    input_length: usize,
}

impl<'a> Lexer<'a> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'a str) -> Self {
        Lexer {
            input: input.char_indices().peekable(),
            input_string: input,
            input_length: input.len(),
        }
    }

    /// One character progresses.
    fn read_char(&mut self) -> (usize, char) {
        self.input.next().unwrap_or((self.input_length, '\u{0}'))
    }

    fn peek(&mut self) -> char {
        match self.input.peek() {
            Some((_, c)) => *c,
            None => '\u{0}',
        }
    }

    #[inline]
    fn end_of_previous_char(&mut self) -> usize {
        if let Some((index, _)) = self.input.peek() {
            *index
        } else {
            self.input_length
        }
    }

    /// Skip blank characters.
    fn skip_whitespace(&mut self) {
        while self.peek().is_ascii_whitespace() {
            self.read_char();
        }
    }

    /// Read one command.
    #[inline]
    fn read_command(&mut self) -> &'a str {
        // Always read at least one character.
        let (start, cur) = self.read_char();

        // If the first char is ASCII, we read until the next non-ASCII character.
        if cur.is_ascii_alphabetic() {
            // Read in all ASCII characters.
            while self.peek().is_ascii_alphabetic() {
                self.read_char();
            }
        }

        // To get the end of the command, we take the index of the next character.
        let end = self.end_of_previous_char();
        // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
        unsafe { self.input_string.get_unchecked(start..end) }
    }

    /// Read one number.
    fn read_number(&mut self) -> (&'a str, Op) {
        // We know that the first character is a digit.
        let (start, _) = self.read_char();

        while {
            let cur = self.peek();
            cur.is_ascii_digit() || matches!(cur, '.' | ',')
        } {
            let (index_before, candidate) = self.read_char();
            // Before we accept the current character, we need to check the next one.
            if matches!(candidate, '.' | ',') && !self.peek().is_ascii_digit() {
                // If the candidate is punctuation and the next character is not a digit,
                // we don't want to include the punctuation.
                // But we do need to return the punctuation as an operator.
                let number = unsafe { self.input_string.get_unchecked(start..index_before) };
                let op = match candidate {
                    '.' => ops::FULL_STOP,
                    ',' => ops::COMMA,
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };
                return (number, op);
            }
        }
        let end = self.end_of_previous_char();
        let number = unsafe { self.input_string.get_unchecked(start..end) };
        (number, ops::NULL)
    }

    /// Read text until the next `}`.
    pub(crate) fn read_text_content(&mut self) -> Option<&'a str> {
        let (mut offset, mut cur) = self.read_char();
        let start = offset;
        let mut brace_count = 1;
        loop {
            if cur == '{' {
                brace_count += 1;
            } else if cur == '}' {
                brace_count -= 1;
            }
            if brace_count <= 0 {
                break;
            }
            // Check for escaped characters.
            if cur == '\\' {
                let (_, cur) = self.read_char();
                // We only allow \{ and \} as escaped characters.
                if !matches!(cur, '{' | '}') {
                    return None;
                }
            }
            if cur == '\u{0}' {
                return None;
            }
            (offset, cur) = self.read_char();
        }
        let end = offset;
        unsafe { Some(self.input_string.get_unchecked(start..end)) }
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> Token<'a> {
        if wants_digit && self.peek().is_ascii_digit() {
            let (start, _) = self.read_char();
            let end = self.end_of_previous_char();
            let num = unsafe { self.input_string.get_unchecked(start..end) };
            return Token::Number(num, ops::NULL);
        }
        self.skip_whitespace();

        let token: Token = match self.peek() {
            '=' => Token::Operator(ops::EQUALS_SIGN),
            ';' => Token::Operator(ops::SEMICOLON),
            ',' => Token::Operator(ops::COMMA),
            '.' => Token::Operator(ops::FULL_STOP),
            '\'' => Token::Prime,
            '(' => Token::Paren(ops::LEFT_PARENTHESIS),
            ')' => Token::Paren(ops::RIGHT_PARENTHESIS),
            '{' => Token::GroupBegin,
            '}' => Token::GroupEnd,
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
                self.read_char(); // Discard the backslash.
                return get_command(self.read_command());
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
        self.read_char(); // Discard the character we just peeked at.
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
