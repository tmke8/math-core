use std::mem;
use std::ops::Range;
use std::str::CharIndices;

use mathml_renderer::symbol::{self, MathMLOperator};

use crate::CommandConfig;
use crate::commands::{get_command, get_text_command};
use crate::environments::Env;
use crate::error::{GetUnwrap, LatexErrKind, LatexError};
use crate::token::{EndToken, Span, TokSpan, Token};

/// Lexer
pub(crate) struct Lexer<'config, 'source>
where
    'config: 'source,
{
    input: CharIndices<'source>,
    peek: (usize, Option<char>),
    input_string: &'source str,
    input_length: usize,
    mode: Mode,
    brace_nesting_level: usize,
    parse_cmd_args: Option<u8>,
    cmd_cfg: Option<&'config CommandConfig>,
}

impl<'config, 'source> Lexer<'config, 'source> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(
        input: &'source str,
        parsing_custom_cmds: bool,
        cmd_cfg: Option<&'config CommandConfig>,
    ) -> Self {
        let mut lexer = Lexer {
            input: input.char_indices(),
            peek: (0, None),
            input_string: input,
            input_length: input.len(),
            mode: Mode::default(),
            brace_nesting_level: 0,
            parse_cmd_args: if parsing_custom_cmds {
                Some(0) // Start counting command arguments.
            } else {
                None
            },
            cmd_cfg,
        };
        lexer.read_char(); // Initialize `peek`.
        lexer
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
    fn read_char(&mut self) -> (usize, Option<char>) {
        mem::replace(
            &mut self.peek,
            self.input
                .next()
                .map(|(idx, ch)| (idx, Some(ch)))
                .unwrap_or((self.input_length, None)),
        )
    }

    /// Skip whitespace characters.
    ///
    /// Returns the span of the first skipped whitespace character, or `None` if there are no
    /// whitespace characters to skip.
    fn skip_whitespace(&mut self) -> Option<Span> {
        let mut span: Option<Span> = None;
        while let (loc, Some(ch)) = self.peek
            && ch.is_ascii_whitespace()
        {
            self.read_char(); // Discard the whitespace character.
            if span.is_none() {
                span = Some(Span::new(loc, loc + ch.len_utf8()));
            }
        }
        span
    }

    /// Read one command.
    #[inline]
    fn read_command(&mut self) -> (&'source str, usize) {
        let start = self.peek.0;

        // Read in all ASCII alphabetic characters.
        while self.peek.1.is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.read_char();
        }

        // Commands may end with a "*".
        if self.peek.1 == Some('*') {
            self.read_char();
        }

        if start == self.peek.0 {
            // Always read at least one character.
            self.read_char();
        }

        // To get the end of the command, we take the index of the next character.
        let end = self.peek.0;
        // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
        (self.input_string.get_unwrap(start..end), end)
    }

    /// Read an environment name.
    ///
    /// Reads ASCII alphanumeric characters (and a few others) until the next `}`.
    /// On success, returns the environment name and the index of the character after the `}`.
    ///
    /// Returns `Err` if there are any disallowed characters before the `}`.
    /// The `Err` contains the span and value of the first disallowed character.
    /// If the end of the input is reached before finding a `}`, the `Err` contains
    /// the span and `None`.
    #[inline]
    fn read_env_name(&mut self) -> Result<(&'source str, usize), (Range<usize>, Option<char>)> {
        // If the first character is not `{`, we read a single character.
        let (loc, first) = self.read_char();
        if first != Some('{') {
            return if first.is_some_and(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '|' | '.' | '-' | ',' | '*' | ':')
            }) {
                // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
                Ok((self.input_string.get_unwrap(loc..self.peek.0), self.peek.0))
            } else {
                Err((loc..(loc + first.map_or(0, |ch| ch.len_utf8())), first))
            };
        }
        let start = self.peek.0;

        while self.peek.1.is_some_and(|ch| {
            ch.is_ascii_alphanumeric()
                || ch.is_ascii_whitespace()
                || matches!(ch, '|' | '.' | '-' | ',' | '*' | ':')
        }) {
            self.read_char();
        }

        // Verify that the environment name is followed by a `}`.
        let (loc, closing) = self.read_char();
        if closing == Some('}') {
            let end = loc;
            // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
            Ok((self.input_string.get_unwrap(start..end), end + 1))
        } else {
            Err((loc..(loc + closing.map_or(0, |ch| ch.len_utf8())), closing))
        }
    }

    pub(crate) fn next_token(&mut self) -> Result<TokSpan<'source>, Box<LatexError>> {
        match self.next_token_internal() {
            LexerResult::Tok(tok) => Ok(tok),
            LexerResult::UnknownCommand(cmd, span) => {
                if self.cmd_cfg.is_some_and(|cfg| cfg.ignore_unknown_commands) {
                    Ok(TokSpan::new(Token::UnknownCommand(cmd), span))
                } else {
                    Err(Box::new(LatexError(
                        span.into(),
                        LatexErrKind::UnknownCommand(cmd.into()),
                    )))
                }
            }
            LexerResult::Err(err) => Err(err),
        }
    }

    pub(crate) fn next_token_no_unknown_command(
        &mut self,
    ) -> Result<TokSpan<'config>, Box<LatexError>> {
        match self.next_token_internal() {
            LexerResult::Tok(tok) => Ok(tok),
            LexerResult::UnknownCommand(cmd, span) => Err(Box::new(LatexError(
                span.into(),
                LatexErrKind::UnknownCommand(cmd.into()),
            ))),
            LexerResult::Err(err) => Err(err),
        }
    }

    fn next_token_internal(&mut self) -> LexerResult<'config, 'source> {
        let text_mode = matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. });
        if let Some(span) = self.skip_whitespace() {
            return LexerResult::Tok(TokSpan::new(Token::Whitespace, span));
        }

        let (loc, ch) = self.read_char();
        let ascii_span = Span::new(loc, loc + 1); // An ASCII character always has length 1.
        let Some(ch) = ch else {
            return LexerResult::Tok(TokSpan::new(Token::Eoi, Span::zero_width(loc)));
        };
        if ch == '%' {
            // Skip comments.
            while self.peek.1 != Some('\n') && self.peek.1.is_some() {
                self.read_char();
            }
            self.read_char(); // Consume the newline character.
            // TODO: use `become` here when stabilized.
            return self.next_token_internal();
        }
        let mut span = ascii_span;
        let tok = match ch {
            '\u{0}' => {
                return LexerResult::Err(Box::new(LatexError(
                    loc..(loc + 1),
                    LatexErrKind::DisallowedChar(ch),
                )));
            }
            '!' => {
                if text_mode {
                    Token::Letter(ch)
                } else {
                    Token::ForceClose(symbol::EXCLAMATION_MARK)
                }
            }
            '"' => Token::Letter('”'),
            '#' => {
                if let Some(num) = &mut self.parse_cmd_args {
                    if let Some(next) = self.peek.1
                        && next.is_ascii_digit()
                    {
                        // In pre-defined commands, `#` is used to denote a parameter.
                        let param_num = (next as u32).wrapping_sub('1' as u32);
                        if !(0..=8).contains(&param_num) {
                            return LexerResult::Err(Box::new(LatexError(
                                (loc + 1)..(loc + 2),
                                LatexErrKind::InvalidParameterNumber,
                            )));
                        }
                        let param_num = param_num as u8;
                        if (param_num + 1) > *num {
                            *num = param_num + 1;
                        }
                        // Discard the digit after `#`.
                        self.read_char();
                        span = span.with_length(2);
                        Token::CustomCmdArg(param_num)
                    } else {
                        let (loc, ch) = self.read_char();
                        if let Some(ch) = ch {
                            return LexerResult::Err(Box::new(LatexError(
                                loc..(loc + ch.len_utf8()),
                                LatexErrKind::InvalidParameterNumber,
                            )));
                        } else {
                            return LexerResult::Err(Box::new(LatexError(
                                loc..loc,
                                LatexErrKind::ExpectedParamNumberGotEOI,
                            )));
                        }
                    }
                } else {
                    return LexerResult::Err(Box::new(LatexError(
                        loc..(loc + 1),
                        LatexErrKind::MacroParameterOutsideCustomCommand,
                    )));
                }
            }
            '&' => Token::NewColumn,
            '\'' => Token::Prime,
            '(' => Token::Open(symbol::LEFT_PARENTHESIS),
            ')' => Token::Close(symbol::RIGHT_PARENTHESIS),
            '*' => {
                if text_mode {
                    Token::Letter(ch)
                } else {
                    Token::ForceBinaryOp(symbol::ASTERISK_OPERATOR.as_op())
                }
            }
            '+' => Token::BinaryOp(symbol::PLUS_SIGN),
            ',' => Token::Punctuation(symbol::COMMA),
            '-' => {
                if text_mode {
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
            '`' => Token::Letter('‘'),
            '{' => {
                if matches!(self.mode, Mode::TextStart) {
                    self.mode = Mode::TextGroup {
                        nesting: self.brace_nesting_level,
                    };
                }
                self.brace_nesting_level += 1;
                Token::GroupBegin
            }
            '|' => Token::Ord(symbol::VERTICAL_LINE),
            '}' => {
                let Some(new_level) = self.brace_nesting_level.checked_sub(1) else {
                    return LexerResult::Err(Box::new(LatexError(
                        loc..(loc + 1),
                        LatexErrKind::UnmatchedClose(EndToken::GroupClose),
                    )));
                };
                self.brace_nesting_level = new_level;
                if let Mode::TextGroup { nesting } = self.mode
                    && nesting == self.brace_nesting_level
                {
                    // We are closing a text group.
                    self.mode = Mode::Math;
                }
                Token::GroupEnd
            }
            '~' => Token::NonBreakingSpace,
            '\\' => {
                let (cmd_string, end) = self.read_command();
                let span = Span::new(loc, end);
                // After a command, all whitespace is skipped, even in text mode.
                self.skip_whitespace();
                return self.parse_command(span, cmd_string);
            }
            c => {
                if c.is_ascii_digit() {
                    Token::Digit(c)
                } else {
                    span = span.with_length(c.len_utf8());
                    Token::Letter(c)
                }
            }
        };
        if matches!(self.mode, Mode::TextStart) {
            // If we didn't go into `Mode::TextGroup` (by reading a `{`),
            // we go back to math mode after reading one token.
            self.mode = Mode::Math;
        }
        LexerResult::Tok(TokSpan::new(tok, span))
    }

    fn parse_command(
        &mut self,
        span: Span,
        cmd_string: &'source str,
    ) -> LexerResult<'config, 'source> {
        let tok: Result<(Token<'config>, Span), LatexError> =
            if matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. }) {
                if let Some(tok) = get_text_command(cmd_string) {
                    Ok((tok, span))
                } else {
                    return LexerResult::UnknownCommand(cmd_string, span);
                }
            } else if let Some(tok) = self
                .cmd_cfg
                .and_then(|custom_cmds| custom_cmds.get_command(cmd_string))
                .or_else(|| get_command(cmd_string))
            {
                Ok((tok, span))
            } else {
                let env_marker = match cmd_string {
                    "begin" => Some(EnvMarker::Begin),
                    "end" => Some(EnvMarker::End),
                    _ => None,
                };
                if let Some(env_marker) = env_marker {
                    'env_name: {
                        // First skip any whitespace.
                        self.skip_whitespace();
                        let group_loc = self.peek.0;
                        // Read the environment name.
                        let (name, end) = match self.read_env_name() {
                            Ok(lit) => lit,
                            Err((span, ch)) => match ch {
                                None => {
                                    break 'env_name Err(LatexError(
                                        span,
                                        LatexErrKind::UnclosedGroup(EndToken::GroupClose),
                                    ));
                                }
                                Some(ch) => {
                                    break 'env_name Err(LatexError(
                                        span,
                                        LatexErrKind::DisallowedChar(ch),
                                    ));
                                }
                            },
                        };
                        // Convert the environment name to the `Env` enum.
                        let Some(env) = Env::from_str(name) else {
                            break 'env_name Err(LatexError(
                                group_loc..end,
                                LatexErrKind::UnknownEnvironment(name.into()),
                            ));
                        };
                        let span = Span::new(span.start(), end);
                        Ok((
                            match env_marker {
                                EnvMarker::Begin => Token::Begin(env),
                                EnvMarker::End => Token::End(env),
                            },
                            span,
                        ))
                    }
                } else {
                    return LexerResult::UnknownCommand(cmd_string, span);
                }
            };
        if matches!(self.mode, Mode::TextStart) {
            // If we didn't go into `Mode::TextGroup` (by reading a `{`),
            // we go back to math mode after reading one token.
            self.mode = Mode::Math;
        }
        if matches!(tok, Ok((Token::Text(_), _))) {
            self.mode = Mode::TextStart;
        }
        match tok {
            Ok((tok, span)) => LexerResult::Tok(TokSpan::new(tok, span)),
            Err(err) => LexerResult::Err(Box::new(err)),
        }
    }
}

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Math,
    /// In text mode, spaces are converted to `Token::Whitespace` and
    /// math commands (like `\sqrt`) don't work. Instead, text commands
    /// (like `\ae`) are recognized.
    TextStart,
    TextGroup {
        nesting: usize, // The nesting level of `{` in the text group.
    },
}

enum EnvMarker {
    Begin = 1,
    End = 2,
}

pub(crate) fn recover_limited_ascii(tok: Token) -> Option<char> {
    const COLON: MathMLOperator = symbol::COLON.as_op();
    const ASTERISK: MathMLOperator = symbol::ASTERISK_OPERATOR.as_op();
    match tok {
        Token::Letter(ch) if ch.is_ascii_alphabetic() || ch == '.' => Some(ch),
        Token::Whitespace => Some(' '),
        Token::Ord(symbol::VERTICAL_LINE) => Some('|'),
        Token::Punctuation(symbol::COMMA) => Some(','),
        Token::BinaryOp(symbol::MINUS_SIGN) => Some('-'),
        Token::ForceBinaryOp(ASTERISK) => Some('*'),
        Token::ForceRelation(COLON) => Some(':'),
        Token::Digit(ch) => Some(ch),
        _ => None,
    }
}

enum LexerResult<'config, 'source> {
    Tok(TokSpan<'config>),
    UnknownCommand(&'source str, Span),
    Err(Box<LatexError>),
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
            ("simple_number", r"3"),
            ("number_with_dot", r"3.14"),
            ("number_with_dot_at_end", r"3.14."),
            ("number_with_two_inner_dots", r"3..14"),
            ("lower_case_latin", r"x"),
            ("lower_case_greek", r"\pi"),
            ("assigment_with_space", r"x = 3.14"),
            ("two_lower_case_greek", r"\alpha\beta"),
            ("simple_expression", r"x+y"),
            ("space_and_number", r"\ 1"),
            ("space_in_text", r"\text{  x   y z}"),
            ("comment", "ab%hello\ncd"),
            ("switch_to_text_mode", r"\prod\text\o\sum"),
            ("switch_to_text_mode_braces", r"\prod\text{\o}\sum"),
            ("custom_space", r"{x\hspace{2em}}"),
            ("hspace_whitespace_in_between", r"\hspace {  4  em } x"),
            ("color", r"{x\color{red} y}"),
            ("color_whitespace", r"{x\color     {red} y}"),
            ("color_newline", "{x\\color\n{red} y}"),
            ("color_one_letter", "{x\\color r y}"),
            ("genfrac_with_parens", r"\genfrac(]{0pt}{2}{a+b}{c+d}"),
            (
                "genfrac_with_one_sided_parens",
                r"\genfrac{}]{0pt}{2}{a+b}{c+d}",
            ),
            ("genfrac_without_parens", r"\genfrac{}{}{0pt}{2}{a+b}{c+d}"),
            ("begin_array", r"\begin{array}{c|c}"),
            ("end_array", r"\end{array}{c|c}"),
        ];

        for (name, problem) in problems.into_iter() {
            let mut lexer = Lexer::new(problem, false, None);
            // Call `lexer.next_token(false)` until we get `Token::EOI`.
            let mut tokens = String::new();
            loop {
                let tokloc = lexer.next_token().unwrap();
                if matches!(tokloc.token(), Token::Eoi) {
                    break;
                }
                let (tok, span) = tokloc.into_parts();
                write!(tokens, "{}:{}: {:?}\n", span.start(), span.end(), tok).unwrap();
            }
            assert_snapshot!(name, &tokens, problem);
        }
    }

    #[test]
    fn test_lexer_errors() {
        let problems = [
            ("unknown_command", r"\unknowncmd + x"),
            ("unexpected_close", r"x + y}"),
            ("missing_brace", r"\begin x + y"),
            ("disallowed_chars", r"\begin{matrix x + y}"),
            (
                "unknown_environment",
                r"\begin{unknownenv} x + y \end{unknownenv}",
            ),
            ("unexpected_close_in_group", r"{x + y}}"),
            ("null_character_in_input", "x + \u{0} + y"),
            ("null_character_in_string_literal", "\\text{\u{0}}"),
        ];
        for (name, problem) in problems.into_iter() {
            let mut lexer = Lexer::new(problem, false, None);
            let mut tokens = String::new();
            let err = loop {
                match lexer.next_token() {
                    Ok(tokloc) => {
                        if matches!(tokloc.token(), Token::Eoi) {
                            break None;
                        }
                    }
                    Err(err) => {
                        break Some(err);
                    }
                }
            };
            let Some(err) = err else {
                panic!("Expected an error in problem: {}", problem);
            };
            write!(
                tokens,
                "Error at {}..{}: {:?}\n",
                err.0.start, err.0.end, err.1
            )
            .unwrap();
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
            if matches!(tokloc.token(), Token::Eoi) {
                break;
            }
            let (tok, span) = tokloc.into_parts();
            write!(tokens, "{}..{}: {:?}\n", span.start(), span.end(), tok).unwrap();
        }
        assert!(matches!(lexer.parse_cmd_args(), Some(3)));
        assert_snapshot!("parsing_custom_commands", tokens, problem);
    }

    #[test]
    fn test_recover_limited_ascii() {
        let input = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,-*:|";
        let mut lexer = Lexer::new(input, false, None);

        let mut output = String::new();
        while let Ok(tokloc) = lexer.next_token() {
            let tok = tokloc.into_token();
            if let Some(ch) = recover_limited_ascii(tok) {
                output.push(ch);
            }
            if matches!(tok, Token::Eoi) {
                break;
            }
        }
        assert_eq!(input, output);
    }
}
