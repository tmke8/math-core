//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!
use bumpalo::collections::String;
use bumpalo::Bump;

use crate::{ops, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'a> {
    input: std::str::Chars<'a>,
    pub(crate) cur: char,
    pub(crate) record_whitespace: bool,
    alloc: &'a Bump,
}

impl<'a> Lexer<'a> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'a str, alloc: &'a Bump) -> Self {
        let mut lexer = Lexer {
            input: input.chars(),
            cur: '\u{0}',
            record_whitespace: false,
            alloc,
        };
        lexer.read_char();
        lexer
    }

    /// One character progresses.
    pub(crate) fn read_char(&mut self) {
        self.cur = self.input.next().unwrap_or('\u{0}');
    }

    /// Skip blank characters.
    fn skip_whitespace(&mut self) {
        if self.record_whitespace {
            return;
        }
        while matches!(self.cur, ' ' | '\t' | '\n' | '\r') {
            self.read_char();
        }
    }

    /// Read one command to a token.
    fn read_command(&mut self) -> Token<'a> {
        let mut command = String::new_in(self.alloc);

        // Always read at least one character.
        self.read_char();
        let first = self.cur;
        command.push(first);

        // Read in all ASCII characters.
        self.read_char();
        while first.is_ascii_alphabetic() && self.cur.is_ascii_alphabetic() {
            command.push(self.cur);
            self.read_char();
        }
        Token::from_command(command)
    }

    /// Read one number into a token.
    fn read_number(&mut self) -> Token<'a> {
        let mut number = String::new_in(self.alloc);
        let mut has_period = false;
        while self.cur.is_ascii_digit() || (self.cur == '.' && !has_period) {
            if self.cur == '.' {
                has_period = true;
            }
            number.push(self.cur);
            self.read_char();
        }
        Token::Number(number)
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self) -> Token<'a> {
        self.skip_whitespace();

        let token = match self.cur {
            '=' => Token::Operator(ops::EQUAL),
            ';' => Token::Operator(ops::SEMICOLON),
            ',' => Token::Operator(ops::COMMA),
            '.' => Token::Operator(ops::DOT),
            '\'' => Token::Operator(ops::APOS),
            '(' => Token::Paren(ops::LEFT_PARENTHESIS),
            ')' => Token::Paren(ops::RIGHT_PARENTHESIS),
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::Paren(ops::LEFT_SQUARE_BRACKET),
            ']' => Token::Paren(ops::RIGHT_SQUARE_BRACKET),
            '|' => Token::Paren(ops::VERTICAL_LINE),
            '+' => Token::Operator(ops::PLUS),
            '-' => Token::Operator(ops::MINUS),
            '*' => Token::Operator(ops::ASTERISK),
            '/' => Token::Operator(ops::SOLIDUS),
            '!' => Token::Operator(ops::EXCLAMATION_MARK),
            '<' => Token::Operator(ops::LT),
            '>' => Token::Operator(ops::GT),
            '_' => Token::Underscore,
            '^' => Token::Circumflex,
            '&' => Token::Ampersand,
            '~' => Token::NonBreakingSpace,
            '\u{0}' => Token::EOF,
            ':' => Token::Colon,
            ' ' => Token::Letter('\u{A0}'),
            '\\' => {
                return self.read_command();
            }
            c => {
                if c.is_ascii_digit() {
                    return self.read_number();
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
    use bumpalo::collections::String;
    use bumpalo::Bump;

    use super::super::token::Token;
    use super::*;

    #[test]
    fn lexer_test() {
        let alloc = &Bump::new();
        let problems = vec![
            (r"3", vec![Token::Number(String::from_str_in("3", alloc))]),
            (r"3.14", vec![Token::Number(String::from_str_in("3.14", alloc))]),
            (
                r"3.14.",
                vec![Token::Number(String::from_str_in("3.14", alloc)), Token::Operator(ops::DOT)],
            ),
            (r"x", vec![Token::Letter('x')]),
            (r"\pi", vec![Token::Letter('π')]),
            (
                r"x = 3.14",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::EQUAL),
                    Token::Number(String::from_str_in("3.14", alloc)),
                ],
            ),
            (r"\alpha\beta", vec![Token::Letter('α'), Token::Letter('β')]),
            (
                r"x+y",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::PLUS),
                    Token::Letter('y'),
                ],
            ),
            (
                r"\ 1",
                vec![Token::Space("1"), Token::Number(String::from_str_in("1", alloc))],
            ),
        ];

        for (problem, answer) in problems.iter() {
            let mut lexer = Lexer::new(problem, alloc);
            for answer in answer.iter() {
                assert_eq!(&lexer.next_token(), answer);
            }
        }
    }
}
