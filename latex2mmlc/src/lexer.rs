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
}

impl<'a> Lexer<'a> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.char_indices(),
            cur: '\u{0}',
        };
        lexer.read_char();
        lexer
    }

    /// One character progresses.
    fn read_char(&mut self) {
        self.cur = self.input.next().unwrap_or((0, '\u{0}')).1;
    }

    /// Skip blank characters.
    fn skip_whitespace(&mut self) {
        while matches!(self.cur, ' ' | '\t' | '\n' | '\r') {
            self.read_char();
        }
    }

    /// Read one command to a token.
    #[inline]
    fn read_command(&mut self) -> &'a str {
        let whole_string: &'a str = self.input.as_str();
        let len = whole_string.len();

        // Always read at least one character.
        let (start, first) = self.input.next().unwrap_or((0, '\u{0}'));

        if !first.is_ascii_alphabetic() {
            let (end, next) = self.input.next().unwrap_or((start + len, '\u{0}'));
            self.cur = next;
            // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
            return unsafe { whole_string.get_unchecked(0..(end - start)) };
        }

        // Read in all ASCII characters.
        let (mut end, mut next) = self.input.next().unwrap_or((start, '\u{0}'));
        while next.is_ascii_alphabetic() {
            (end, next) = self.input.next().unwrap_or((start + len, '\u{0}'));
        }
        self.cur = next;
        // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
        unsafe { whole_string.get_unchecked(0..(end - start)) }
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
    pub(crate) fn read_text_content(&mut self, whitespace: WhiteSpace) -> Option<String> {
        let mut text = String::new();
        if matches!(whitespace, WhiteSpace::Skip) {
            self.skip_whitespace();
        }
        while self.cur != '}' {
            if self.cur == '\u{0}' {
                return None;
            }
            if matches!(whitespace, WhiteSpace::Convert) && self.cur == ' ' {
                text.push('\u{A0}')
            } else {
                text.push(self.cur);
            }
            self.read_char();
            if matches!(whitespace, WhiteSpace::Skip) {
                self.skip_whitespace();
            }
        }
        self.read_char(); // Discard the closing brace.
        Some(text)
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> Token<'a> {
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

pub(crate) enum WhiteSpace {
    Skip,
    Record,
    Convert,
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
