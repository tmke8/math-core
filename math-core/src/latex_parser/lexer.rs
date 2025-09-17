use std::mem;
use std::num::NonZero;
use std::str::CharIndices;

use super::commands::{get_command, get_text_command};
use super::environments::Env;
use super::error::{GetUnwrap, LatexErrKind, LatexError};
use super::token::{Digit, TokLoc, TokResult, Token};
use crate::CustomCmds;
use crate::mathml_renderer::symbol;

/// Lexer
pub(crate) struct Lexer<'config, 'source>
where
    'config: 'source,
{
    input: CharIndices<'source>,
    peek: (usize, char),
    input_string: &'source str,
    input_length: usize,
    text_mode: bool,
    parse_cmd_args: Option<u8>,
    custom_cmds: Option<&'config CustomCmds>,
}

impl<'config, 'source> Lexer<'config, 'source> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(
        input: &'source str,
        parsing_custom_cmds: bool,
        custom_cmds: Option<&'config CustomCmds>,
    ) -> Self {
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

    #[inline]
    pub(super) fn turn_off_text_mode(&mut self) {
        self.text_mode = false;
    }

    #[inline]
    pub(super) fn input_length(&self) -> usize {
        self.input_length
    }

    #[inline]
    pub(crate) fn parse_cmd_args(&self) -> Option<u8> {
        self.parse_cmd_args
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
    pub(super) fn read_ascii_text_group(&mut self) -> Option<&'source str> {
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
    pub(super) fn is_next_digit(&mut self) -> bool {
        if !self.text_mode {
            self.skip_whitespace();
        }
        self.peek.1.is_ascii_digit()
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    pub(super) fn read_group(
        &mut self,
    ) -> Result<Vec<TokResult<'config, 'config>>, LatexError<'source>> {
        let mut tokens = Vec::new();
        // Set the initial nesting level to 1.
        let mut brace_nesting_level: usize = 1;
        loop {
            let tokloc = self.next_token()?;
            match tokloc.1 {
                Token::GroupBegin => {
                    brace_nesting_level += 1;
                }
                Token::GroupEnd => {
                    // Decrease the nesting level.
                    // This cannot underflow because we started at 1 and stop
                    // when it reaches 0.
                    brace_nesting_level -= 1;
                    // If the nesting level is 0, we stop reading.
                    if brace_nesting_level == 0 {
                        // We break directly without pushing the `}` token.
                        break;
                    }
                }
                Token::Eof => {
                    return Err(LatexError(tokloc.0, LatexErrKind::UnclosedGroup(tokloc.1)));
                }
                _ => {}
            }
            tokens.push(TokResult(tokloc.0, Ok(tokloc.1)));
        }
        Ok(tokens)
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self) -> Result<TokLoc<'config>, LatexError<'source>> {
        if let Some(loc) = self.skip_whitespace() {
            if self.text_mode {
                return Ok((loc.get(), Token::Whitespace));
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
            '\u{0}' => Token::Eof,
            ' ' => Token::Letter('\u{A0}'),
            '!' => Token::Ord(symbol::EXCLAMATION_MARK),
            '#' => {
                if self.parse_cmd_args.is_some() && self.peek.1.is_ascii_digit() {
                    // In pre-defined commands, `#` is used to denote a parameter.
                    let next = self.read_char().1;
                    let param_num = (next as u32).wrapping_sub('1' as u32);
                    if !(0..=8).contains(&param_num) {
                        return Err(LatexError(loc, LatexErrKind::InvalidParameterNumber));
                    }
                    let param_num = param_num as u8;
                    if let Some(num) = self.parse_cmd_args.as_mut() {
                        if (param_num + 1) > *num {
                            *num = param_num + 1;
                        }
                    }
                    Token::CustomCmdArg(param_num)
                } else {
                    Token::Letter('#')
                }
            }
            '&' => Token::Ampersand,
            '\'' => Token::Prime,
            '(' => Token::Open(symbol::LEFT_PARENTHESIS),
            ')' => Token::Close(symbol::RIGHT_PARENTHESIS),
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
            '/' => Token::Ord(symbol::SOLIDUS),
            ':' => Token::ForceRelation(symbol::COLON.as_op()),
            ';' => Token::Punctuation(symbol::SEMICOLON),
            '<' => Token::OpLessThan,
            '=' => Token::Relation(symbol::EQUALS_SIGN),
            '>' => Token::OpGreaterThan,
            '[' => Token::SquareBracketOpen,
            ']' => Token::SquareBracketClose,
            '^' => Token::Circumflex,
            '_' => Token::Underscore,
            '{' => Token::GroupBegin,
            '|' => Token::Ord(symbol::VERTICAL_LINE),
            '}' => Token::GroupEnd,
            '~' => Token::NonBreakingSpace,
            '\\' => {
                let cmd_string = self.read_command();
                if self.text_mode {
                    // After a command, all whitespace is skipped, even in text mode.
                    // This is done automatically in non-text-mode, but for text
                    // mode we need to do it manually.
                    self.skip_whitespace();
                }
                return self.parse_command(loc, cmd_string);
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
        Ok((loc, tok))
    }

    fn parse_command(
        &mut self,
        loc: usize,
        cmd_string: &'source str,
    ) -> Result<TokLoc<'config>, LatexError<'source>> {
        let tok = if self.text_mode {
            let Some(tok) = get_text_command(cmd_string) else {
                return Err(LatexError(loc, LatexErrKind::UnknownCommand(cmd_string)));
            };
            tok
        } else {
            let env_marker = match cmd_string {
                "begin" => Some(EnvMarker::Begin),
                "end" => Some(EnvMarker::End),
                _ => None,
            };
            if let Some(env_marker) = env_marker {
                // Read the environment name.
                // First skip any whitespace.
                self.skip_whitespace();
                // Next character must be `{`.
                let (new_loc, next_char) = self.read_char();
                if next_char != '{' {
                    return Err(LatexError(new_loc, LatexErrKind::MissingBrace(next_char)));
                }
                // Read the text until the next `}`.
                let Some(env_name) = self.read_ascii_text_group() else {
                    return Err(LatexError(new_loc, LatexErrKind::DisallowedChars));
                };
                // Convert the environment name to the `Env` enum.
                let Some(env) = Env::from_str(env_name) else {
                    return Err(LatexError(
                        new_loc,
                        LatexErrKind::UnknownEnvironment(env_name),
                    ));
                };
                // Return the `\begin{env}` or `\end{env}` token.
                match env_marker {
                    EnvMarker::Begin => Token::Begin(env),
                    EnvMarker::End => Token::End(env),
                }
            } else {
                let Some(tok) = self
                    .custom_cmds
                    .and_then(|custom_cmds| custom_cmds.get_command(cmd_string))
                    .or_else(|| get_command(cmd_string))
                else {
                    return Err(LatexError(loc, LatexErrKind::UnknownCommand(cmd_string)));
                };
                tok
            }
        };
        if matches!(tok, Token::Text(_)) {
            self.text_mode = true;
        }
        Ok((loc, tok))
    }
}

enum EnvMarker {
    Begin = 1,
    End = 2,
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
                let tokloc = lexer.next_token().unwrap();
                if matches!(tokloc.1, Token::Eof) {
                    break;
                }
                let (loc, tok) = tokloc;
                write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
            }
            assert_snapshot!(name, &tokens, problem);
        }
    }

    #[test]
    fn test_read_group() {
        let problems = [
            ("simple_group", r"{x+y}"),
            ("group_followed", r"{x+y} b"),
            ("nested_group", r"{x + {y - z}} c"),
            ("unclosed_group", r"{x + y"),
            ("unclosed_nested_group", r"{x + {y + z}"),
            ("too_many_closes", r"{x + y} + z}"),
            ("empty_group", r"{} d"),
            ("group_with_begin", r"{\begin{matrix}}"),
            ("early_error", r"{x + \unknowncmd + y}"),
        ];

        for (name, problem) in problems.into_iter() {
            let mut lexer = Lexer::new(problem, false, None);
            // Check that the first token is `GroupBegin`.
            assert!(matches!(lexer.next_token().unwrap().1, Token::GroupBegin));
            let tokens = match lexer.read_group() {
                Ok(tokens) => {
                    let mut token_str = String::new();
                    for TokResult(loc, tok) in tokens {
                        write!(token_str, "{}: {:?}\n", loc, tok.unwrap()).unwrap();
                    }
                    token_str
                }
                Err(err) => format!("Error at {}: {:?}", err.0, err.1),
            };
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
            let tokloc = lexer.next_token().unwrap();
            if matches!(tokloc.1, Token::Eof) {
                break;
            }
            let (loc, tok) = tokloc;
            write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
        }
        assert!(matches!(lexer.parse_cmd_args(), Some(3)));
        assert_snapshot!("parsing_custom_commands", tokens, problem);
    }
}
