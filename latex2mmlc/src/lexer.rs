//! Lexer
//!
//! - Input: `String`
//! - Output: `Vec<Token>`
//!

use std::mem;
use std::num::NonZero;
use std::str::CharIndices;

use crate::attribute::ParenAttr;
use crate::commands::get_command;
use crate::error::GetUnwrap;
use crate::token::TokLoc;
use crate::{ops, token::Token};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'source> {
    input: CharIndices<'source>,
    peek: (usize, char),
    input_string: &'source str,
    pub input_length: usize,
    pub text_mode: bool,
}

impl<'source> Lexer<'source> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(input: &'source str) -> Self {
        let mut lexer = Lexer {
            input: input.char_indices(),
            peek: (0, '\u{0}'),
            input_string: input,
            input_length: input.len(),
            text_mode: false,
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

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) -> Option<NonZero<usize>> {
        let mut skipped = None;
        while self.peek.1.is_ascii_whitespace() {
            let (loc, _) = self.read_char();
            // This is technically wrong because there can be whitespace at position 0,
            // but we are only recording whitespace in text mode, which is started by
            // the `\text` command, so at position 0 we will never we in text mode.
            skipped = NonZero::<usize>::new(loc);
        }
        skipped
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
        self.input_string.get_unwrap(start..end)
    }

    /// Read one number.
    #[inline]
    fn read_number(&mut self, start: usize) -> Token<'source> {
        while {
            let cur = self.peek.1;
            cur.is_ascii_digit() || Punctuation::from_char(cur).is_some()
        } {
            let (index_before, candidate) = self.read_char();
            // Before we accept the current character, we need to check the next one.
            if !self.peek.1.is_ascii_digit() {
                if let Some(punctuation) = Punctuation::from_char(candidate) {
                    // If the candidate is punctuation and the next character is not a digit,
                    // we don't want to include the punctuation.
                    // But we do need to return the punctuation as an operator.
                    let number = self.input_string.get_unwrap(start..index_before);
                    return match punctuation {
                        Punctuation::Dot => Token::NumberWithDot(number),
                        Punctuation::Comma => Token::NumberWithComma(number),
                    };
                }
            }
        }
        let end = self.peek.0;
        let number = self.input_string.get_unwrap(start..end);
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
                return Some(self.input_string.get_unwrap(start..end));
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
    pub(crate) fn next_token(&mut self, wants_digit: bool) -> TokLoc<'source> {
        if let Some(loc) = self.skip_whitespace() {
            if self.text_mode {
                return TokLoc(loc.get(), Token::Whitespace);
            }
        }
        if wants_digit && self.peek.1.is_ascii_digit() {
            let (start, _) = self.read_char();
            let end = self.peek.0;
            let num = self.input_string.get_unwrap(start..end);
            return TokLoc(start, Token::Number(num));
        }

        let (loc, ch) = self.read_char();
        let tok = match ch {
            '\u{0}' => Token::EOF,
            ' ' => Token::Letter('\u{A0}'),
            '!' => Token::Operator(ops::EXCLAMATION_MARK),
            '&' => Token::Ampersand,
            '\'' => Token::Prime,
            '(' => Token::Paren(ops::LEFT_PARENTHESIS, None),
            ')' => Token::Paren(ops::RIGHT_PARENTHESIS, None),
            '*' => Token::Operator(ops::ASTERISK),
            '+' => Token::Operator(ops::PLUS_SIGN),
            ',' => Token::Operator(ops::COMMA),
            '-' => Token::Operator(ops::MINUS_SIGN),
            '/' => Token::Paren(ops::SOLIDUS, Some(ParenAttr::Ordinary)),
            ':' => Token::Colon,
            ';' => Token::Operator(ops::SEMICOLON),
            '<' => Token::OpLessThan,
            '=' => Token::Operator(ops::EQUALS_SIGN),
            '>' => Token::OpGreaterThan,
            '[' => Token::SquareBracketOpen,
            ']' => Token::SquareBracketClose,
            '^' => Token::Circumflex,
            '_' => Token::Underscore,
            '{' => Token::GroupBegin,
            '|' => Token::Paren(ops::VERTICAL_LINE, Some(ParenAttr::Ordinary)),
            '}' => Token::GroupEnd,
            '~' => Token::NonBreakingSpace,
            '\\' => {
                let cmd = get_command(self.read_command());
                if self.text_mode {
                    // After a command, all whitespace is skipped, even in text mode.
                    self.skip_whitespace();
                }
                cmd
            }
            c => {
                if c.is_ascii_digit() {
                    self.read_number(loc)
                } else {
                    // Some symbols like '.' and '/' are considered operators by the MathML Core spec,
                    // but in LaTeX they behave like normal identifiers (they are in the "ordinary" class 0).
                    // One might think that they could be rendered as `<mo>` with custom spacing,
                    // but then they still interact with other operators in ways that are not correct.
                    Token::Letter(c)
                }
            }
        };
        TokLoc(loc, tok)
    }
}

enum Punctuation {
    Dot,
    Comma,
}

impl Punctuation {
    fn from_char(c: char) -> Option<Self> {
        match c {
            ops::FULL_STOP => Some(Self::Dot),
            ',' => Some(Self::Comma),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use insta::assert_snapshot;

    use super::super::token::Token;
    use super::*;

    #[test]
    fn lexer_test() {
        let problems = [
            ("simple_number", r"3", false),
            ("number_with_dot", r"3.14", false),
            ("number_with_dot_at_end", r"3.14.", false),
            ("number_with_two_inner_dots", r"3..14", false),
            ("lower_case_latin", r"x", false),
            ("lower_case_greek", r"\pi", false),
            ("assigment_with_space", r"x = 3.14", false),
            ("two_lower_case_greek", r"\alpha\beta", false),
            ("simple_expression", r"x+y", false),
            ("space_and_number", r"\ 1", false),
            ("space_in_text", r"  x   y z", true),
        ];

        for (name, problem, text_mode) in problems.into_iter() {
            let mut lexer = Lexer::new(problem);
            lexer.text_mode = text_mode;
            // Call `lexer.next_token(false)` until we get `Token::EOF`.
            let mut tokens = String::new();
            if text_mode {
                write!(tokens, "(text mode)\n").unwrap();
            }
            loop {
                let tokloc = lexer.next_token(false);
                if matches!(tokloc.token(), Token::EOF) {
                    break;
                }
                let TokLoc(loc, tok) = tokloc;
                write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
            }
            assert_snapshot!(name, &tokens, problem);
        }
    }
}
