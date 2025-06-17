use std::mem;
use std::num::NonZero;
use std::str::CharIndices;

use super::commands::{get_command, get_text_command};
use super::error::GetUnwrap;
use super::token::{Digit, TokLoc, Token};
use crate::CustomCmds;
use crate::mathml_renderer::symbol;

/// Lexer
pub(crate) struct Lexer<'source> {
    input: CharIndices<'source>,
    peek: (usize, char),
    input_string: &'source str,
    pub input_length: usize,
    pub text_mode: bool,
    pub parse_cmd_args: Option<usize>,
    custom_cmds: Option<&'source CustomCmds>,
}

impl<'source> Lexer<'source> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new<'config>(
        input: &'source str,
        parsing_custom_cmds: bool,
        custom_cmds: Option<&'config CustomCmds>,
    ) -> Self
    where
        'config: 'source,
    {
        let mut lexer = Lexer {
            input: input.char_indices(),
            peek: (0, '\u{0}'),
            input_string: input,
            input_length: input.len(),
            text_mode: false,
            parse_cmd_args: if parsing_custom_cmds {
                Some(0) // Start counting command arguments.
            } else {
                None
            },
            custom_cmds,
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

    /// Read ASCII alphanumeric characters until the next `}`.
    ///
    /// Returns `None` if there are any non-alphanumeric characters before the `}`.
    #[inline]
    pub(crate) fn read_ascii_text_group(&mut self) -> Option<&'source str> {
        let start = self.peek.0;

        while self.peek.1.is_ascii_alphanumeric()
            || self.peek.1.is_ascii_whitespace()
            || matches!(self.peek.1, '|' | '.' | '-' | ',' | '*' | ':')
        {
            self.read_char();
        }

        // Verify that the environment name is followed by a `}`.
        let closing = self.read_char();
        if closing.1 == '}' {
            let end = closing.0;
            // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
            Some(self.input_string.get_unwrap(start..end))
        } else {
            None
        }
    }

    /// Check if the next character is a digit.
    pub(crate) fn is_next_digit(&mut self) -> bool {
        if !self.text_mode {
            self.skip_whitespace();
        }
        self.peek.1.is_ascii_digit()
    }

    /// Generate the next token.
    ///
    /// If `wants_arg` is `true`, the lexer will not collect digits into a number token,
    /// but rather immediately return a single digit as a number token.
    pub(crate) fn next_token(&mut self) -> TokLoc<'source> {
        if let Some(loc) = self.skip_whitespace() {
            if self.text_mode {
                return TokLoc(loc.get(), Token::Whitespace);
            }
        }

        let (loc, ch) = self.read_char();
        if ch == '%' {
            // Skip comments.
            while self.peek.1 != '\n' && self.peek.1 != '\u{0}' {
                self.read_char();
            }
            return self.next_token();
        }
        let tok = match ch {
            '\u{0}' => Token::EOF,
            ' ' => Token::Letter('\u{A0}'),
            '!' => Token::Punctuation(symbol::EXCLAMATION_MARK),
            '#' => {
                if self.parse_cmd_args.is_some() && self.peek.1.is_ascii_digit() {
                    // In pre-defined commands, `#` is used to denote a parameter.
                    let next = self.read_char().1;
                    let param_num = (next as u32).wrapping_sub('0' as u32) as usize;
                    if let Some(num) = self.parse_cmd_args.as_mut() {
                        if param_num > *num {
                            *num = param_num;
                        }
                    }
                    Token::CustomCmdArg(param_num.saturating_sub(1))
                } else {
                    Token::Letter('#')
                }
            }
            '&' => Token::Ampersand,
            '\'' => Token::Prime,
            '(' => Token::Delimiter(symbol::LEFT_PARENTHESIS),
            ')' => Token::Delimiter(symbol::RIGHT_PARENTHESIS),
            '*' => {
                if self.text_mode {
                    Token::Letter(ch)
                } else {
                    Token::BinaryOp(symbol::ASTERISK_OPERATOR)
                }
            }
            '+' => Token::BinaryOp(symbol::PLUS_SIGN),
            ',' => Token::Punctuation(symbol::COMMA),
            '-' => {
                if self.text_mode {
                    Token::Letter(ch)
                } else {
                    Token::BinaryOp(symbol::MINUS_SIGN)
                }
            }
            '/' => Token::Delimiter(symbol::SOLIDUS),
            ':' => Token::Colon,
            ';' => Token::Punctuation(symbol::SEMICOLON),
            '<' => Token::OpLessThan,
            '=' => Token::Relation(symbol::EQUALS_SIGN),
            '>' => Token::OpGreaterThan,
            '[' => Token::SquareBracketOpen,
            ']' => Token::SquareBracketClose,
            '^' => Token::Circumflex,
            '_' => Token::Underscore,
            '{' => Token::GroupBegin,
            '|' => Token::Delimiter(symbol::VERTICAL_LINE),
            '}' => Token::GroupEnd,
            '~' => Token::NonBreakingSpace,
            '\\' => {
                let cmd_string = self.read_command();
                let tok = if self.text_mode {
                    // After a command, all whitespace is skipped, even in text mode.
                    self.skip_whitespace();
                    get_text_command(cmd_string)
                } else {
                    self.custom_cmds
                        .and_then(|custom_cmds| custom_cmds.get_command(cmd_string))
                        .unwrap_or_else(|| get_command(cmd_string))
                };
                if matches!(tok, Token::Text(_)) {
                    self.text_mode = true;
                }
                tok
            }
            c => {
                if let Ok(digit) = Digit::try_from(c) {
                    Token::Number(digit)
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
            ("comment", "ab%hello\ncd", false),
            ("switch_to_text_mode", r"\text\o", false),
        ];

        for (name, problem, text_mode) in problems.into_iter() {
            let mut lexer = Lexer::new(problem, false, None);
            lexer.text_mode = text_mode;
            // Call `lexer.next_token(false)` until we get `Token::EOF`.
            let mut tokens = String::new();
            if text_mode {
                write!(tokens, "(text mode)\n").unwrap();
            }
            loop {
                let tokloc = lexer.next_token();
                if matches!(tokloc.token(), Token::EOF) {
                    break;
                }
                let TokLoc(loc, tok) = tokloc;
                write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
            }
            assert_snapshot!(name, &tokens, problem);
        }
    }

    #[test]
    fn test_parsing_custom_commands() {
        let parsing_custom_cmds = true;
        let problem = r"\frac{#1}{#2} + \sqrt{#3}";
        let mut lexer = Lexer::new(problem, parsing_custom_cmds, None);
        let mut tokens = String::new();
        loop {
            let tokloc = lexer.next_token();
            if matches!(tokloc.token(), Token::EOF) {
                break;
            }
            let TokLoc(loc, tok) = tokloc;
            write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
        }
        assert_snapshot!("parsing_custom_commands", tokens, problem);
    }
}
