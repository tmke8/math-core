//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!

use std::mem;
use std::str::CharIndices;

use crate::commands::get_command;
use crate::{ops, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'source> {
    input: CharIndices<'source>,
    peek: (usize, char),
    input_string: &'source str,
    input_length: usize,
}

impl<'source> Lexer<'source> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'source str) -> Self {
        let mut lexer = Lexer {
            input: input.char_indices(),
            peek: (0, '\u{0}'),
            input_string: input,
            input_length: input.len(),
        };
        lexer.read_char(); // Initialize `peek`.
        lexer
    }

    /// One character progresses.
    fn read_char(&mut self) -> (usize, char) {
        mem::replace(
            &mut self.peek,
            self.input.next().unwrap_or((self.input_length, '\u{0}')),
        )
    }

    /// Skip blank characters.
    fn skip_whitespace(&mut self) {
        while self.peek.1.is_ascii_whitespace() {
            self.read_char();
        }
    }

    /// Read one command.
    #[inline]
    fn read_command(&mut self) -> &'source str {
        let start = self.peek.0;

        // Read in all ASCII characters.
        while self.peek.1.is_ascii_alphabetic() {
            self.read_char();
        }

        if start == self.peek.0 {
            // Always read at least one character.
            self.read_char();
        }

        // To get the end of the command, we take the index of the next character.
        let end = self.peek.0;
        // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
        unsafe { self.input_string.get_unchecked(start..end) }
    }

    /// Read one number.
    #[inline]
    fn read_number(&mut self, start: usize) -> Token<'source> {
        while {
            let cur = self.peek.1;
            cur.is_ascii_digit() || matches!(cur, '.' | ',')
        } {
            let (index_before, candidate) = self.read_char();
            // Before we accept the current character, we need to check the next one.
            if matches!(candidate, '.' | ',') && !self.peek.1.is_ascii_digit() {
                // If the candidate is punctuation and the next character is not a digit,
                // we don't want to include the punctuation.
                // But we do need to return the punctuation as an operator.
                let number = unsafe { self.input_string.get_unchecked(start..index_before) };
                return match candidate {
                    '.' => Token::NumberWithDot(number),
                    ',' => Token::NumberWithComma(number),
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };
            }
        }
        let end = self.peek.0;
        let number = unsafe { self.input_string.get_unchecked(start..end) };
        Token::Number(number)
    }

    /// Read text until the next `}`.
    #[inline]
    pub(crate) fn read_text_content(&mut self) -> Option<&'source str> {
        let mut brace_count = 1;
        let start = self.peek.0;

        loop {
            let (end, cur) = self.read_char();
            if cur == '{' {
                brace_count += 1;
            } else if cur == '}' {
                brace_count -= 1;
            }
            if brace_count <= 0 {
                return unsafe { Some(self.input_string.get_unchecked(start..end)) };
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
        }
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> Token<'source> {
        self.skip_whitespace();
        if wants_digit && self.peek.1.is_ascii_digit() {
            let (start, _) = self.read_char();
            let end = self.peek.0;
            let num = unsafe { self.input_string.get_unchecked(start..end) };
            return Token::Number(num);
        }

        match self.read_char() {
            (_, '=') => Token::Operator(ops::EQUALS_SIGN),
            (_, ';') => Token::Operator(ops::SEMICOLON),
            (_, ',') => Token::Operator(ops::COMMA),
            (_, '.') => Token::Operator(ops::FULL_STOP),
            (_, '\'') => Token::Prime,
            (_, '(') => Token::Paren(ops::LEFT_PARENTHESIS),
            (_, ')') => Token::Paren(ops::RIGHT_PARENTHESIS),
            (_, '{') => Token::GroupBegin,
            (_, '}') => Token::GroupEnd,
            (_, '[') => Token::Paren(ops::LEFT_SQUARE_BRACKET),
            (_, ']') => Token::SquareBracketClose,
            (_, '|') => Token::Paren(ops::VERTICAL_LINE),
            (_, '+') => Token::Operator(ops::PLUS_SIGN),
            (_, '-') => Token::Operator(ops::MINUS_SIGN),
            (_, '*') => Token::Operator(ops::ASTERISK),
            (_, '/') => Token::Operator(ops::SOLIDUS),
            (_, '!') => Token::Operator(ops::EXCLAMATION_MARK),
            (_, '<') => Token::Operator(ops::LESS_THAN_SIGN),
            (_, '>') => Token::Operator(ops::GREATER_THAN_SIGN),
            (_, '_') => Token::Underscore,
            (_, '^') => Token::Circumflex,
            (_, '&') => Token::Ampersand,
            (_, '~') => Token::NonBreakingSpace,
            (_, '\u{0}') => Token::EOF,
            (_, ':') => Token::Colon,
            (_, ' ') => Token::Letter('\u{A0}'),
            (_, '\\') => get_command(self.read_command()),
            (start, c) => {
                if c.is_ascii_digit() {
                    self.read_number(start)
                } else if c.is_ascii_alphabetic() {
                    Token::Letter(c)
                } else {
                    Token::NormalLetter(c)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::token::Token;
    use super::*;

    #[test]
    fn lexer_test() {
        let problems = vec![
            (r"3", vec![Token::Number("3")]),
            (r"3.14", vec![Token::Number("3.14")]),
            (r"3.14.", vec![Token::NumberWithDot("3.14")]),
            (
                r"3..14",
                vec![
                    Token::NumberWithDot("3"),
                    Token::Operator(ops::FULL_STOP),
                    Token::Number("14"),
                ],
            ),
            (r"x", vec![Token::Letter('x')]),
            (r"\pi", vec![Token::Letter('π')]),
            (
                r"x = 3.14",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::EQUALS_SIGN),
                    Token::Number("3.14"),
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
            (r"\ 1", vec![Token::Space("1"), Token::Number("1")]),
        ];

        for (problem, answer) in problems.iter() {
            let mut lexer = Lexer::new(problem);
            for answer in answer.iter() {
                assert_eq!(&lexer.next_token(false), answer);
            }
        }
    }
}
