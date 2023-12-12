//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!

use super::{ops, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'a> {
    input: std::str::Chars<'a>,
    pub(crate) cur: char,
    pub(crate) record_whitespace: bool,
}

impl<'a> Lexer<'a> {
    /// 入力ソースコードを受け取り Lexer インスタンスを生成する.
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.chars(),
            cur: '\u{0}',
            record_whitespace: false,
        };
        lexer.read_char();
        lexer
    }

    /// 1 文字進む.
    pub(crate) fn read_char(&mut self) {
        self.cur = self.input.next().unwrap_or('\u{0}');
    }

    /// 空白文字をスキップする.
    fn skip_whitespace(&mut self) {
        if self.record_whitespace {
            return;
        }
        while matches!(self.cur, ' ' | '\t' | '\n' | '\r') {
            self.read_char();
        }
    }

    /// コマンド一つ分を読み込みトークンに変換する.
    fn read_command(&mut self) -> Token {
        let mut command = String::new();
        // 1 文字は確実に読む
        self.read_char();
        let first = self.cur;
        command.push(first);
        self.read_char();
        // ASCII アルファベットなら続けて読む
        while first.is_ascii_alphabetic() && self.cur.is_ascii_alphabetic() {
            command.push(self.cur);
            self.read_char();
        }

        Token::from_command(command)
    }

    /// 数字一つ分を読み込みトークンに変換する.
    fn read_number(&mut self) -> Token {
        let mut number = String::new();
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

    /// 次のトークンを生成する.
    pub(crate) fn next_token(&mut self) -> Token {
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
    use super::super::token::Token;
    use super::*;

    #[test]
    fn lexer_test() {
        let problems = vec![
            (r"3", vec![Token::Number("3".to_owned())]),
            (r"3.14", vec![Token::Number("3.14".to_owned())]),
            (
                r"3.14.",
                vec![Token::Number("3.14".to_owned()), Token::Operator(ops::DOT)],
            ),
            (r"x", vec![Token::Letter('x')]),
            (r"\pi", vec![Token::Letter('π')]),
            (
                r"x = 3.14",
                vec![
                    Token::Letter('x'),
                    Token::Operator(ops::EQUAL),
                    Token::Number("3.14".to_owned()),
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
                vec![Token::Space("1"), Token::Number("1".to_owned())],
            ),
        ];

        for (problem, answer) in problems.iter() {
            let mut lexer = Lexer::new(problem);
            for answer in answer.iter() {
                assert_eq!(&lexer.next_token(), answer);
            }
        }
    }
}
