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
    input: std::str::Chars<'a>,
    cur: char,
}

impl<'a> Lexer<'a> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.chars(),
            cur: '\u{0}',
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
        while matches!(self.cur, ' ' | '\t' | '\n' | '\r') {
            self.read_char();
        }
    }

    /// Read one command to a token.
    fn read_command(&mut self) -> String {
        let mut command = String::new();

        // Always read at least one character.
        self.read_char();
        let first = self.cur;
        command.push(first);

        self.read_char();
        if !first.is_ascii_alphabetic() {
            return command;
        }

        // Read in all ASCII characters.
        while self.cur.is_ascii_alphabetic() {
            command.push(self.cur);
            self.read_char();
        }
        command
    }

    /// Read one number into a token.
    fn read_number(&mut self) -> (String, Op) {
        let mut number = String::new();
        let mut last = self.cur;
        self.read_char();
        while last.is_ascii_digit() || (matches!(last, '.' | ',') && self.cur.is_ascii_digit()) {
            number.push(last);
            if self.cur.is_ascii_digit() || matches!(self.cur, '.' | ',') {
                last = self.cur;
                self.read_char();
            } else {
                return (number, ops::NULL);
            }
        }
        if matches!(last, '.' | ',') {
            return (number, Op(last));
        }
        (number, ops::NULL)
    }

    /// Read text until the next `}`.
    pub(crate) fn read_text_content(&mut self, skip_whitespace: bool) -> Option<String> {
        let mut text = String::new();
        if skip_whitespace {
            self.skip_whitespace();
        }
        while self.cur != '}' {
            if self.cur == '\u{0}' {
                return None;
            }
            text.push(self.cur);
            self.read_char();
            if skip_whitespace {
                self.skip_whitespace();
            }
        }
        self.read_char(); // Discard the closing brace.
        Some(text)
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> Token {
        if wants_digit && self.cur.is_ascii_digit() {
            let num = self.cur;
            self.read_char();
            return Token::Number(num.to_string(), ops::NULL);
        }
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
            (r"3", vec![Token::Number("3".to_owned(), ops::NULL)]),
            (r"3.14", vec![Token::Number("3.14".to_owned(), ops::NULL)]),
            (r"3.14.", vec![Token::Number("3.14".to_owned(), ops::DOT)]),
            (
                r"3..14",
                vec![
                    Token::Number("3".to_owned(), ops::DOT),
                    Token::Operator(ops::DOT),
                    Token::Number("14".to_owned(), ops::NULL),
                ],
            ),
            (r"x", vec![Token::Letter('x')]),
            (r"\pi", vec![Token::Letter('π')]),
            (
                r"x = 3.14",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::EQUAL),
                    Token::Number("3.14".to_owned(), ops::NULL),
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
                vec![Token::Space("1"), Token::Number("1".to_owned(), ops::NULL)],
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
