use std::mem;
use std::num::NonZeroUsize;
use std::str::CharIndices;

use mathml_renderer::symbol::{self, MathMLOperator};

use crate::CustomCmds;
use crate::commands::{get_command, get_text_command};
use crate::environments::Env;
use crate::error::{GetUnwrap, LatexErrKind, LatexError};
use crate::token::{TokLoc, Token};

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
            custom_cmds,
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
    fn skip_whitespace(&mut self) -> Option<NonZeroUsize> {
        let mut skipped = None;
        while self.peek.1.is_some_and(|ch| ch.is_ascii_whitespace()) {
            let (loc, _) = self.read_char();
            // This is technically wrong because there can be whitespace at position 0,
            // but we are only recording whitespace in text mode, which is started by
            // the `\text` command, so at position 0 we will never we in text mode.
            skipped = NonZeroUsize::new(loc);
        }
        skipped
    }

    /// Read one command.
    #[inline]
    fn read_command(&mut self) -> &'source str {
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
        self.input_string.get_unwrap(start..end)
    }

    /// Read ASCII alphanumeric characters (and a few others) until the next `}`.
    ///
    /// Returns `Err` if there are any disallowed characters before the `}`.
    /// The `Err` contains the location and character of the first disallowed character.
    /// If the end of the input is reached before finding a `}`, the `Err` contains
    /// the location and `None`.
    #[inline]
    fn read_ascii_text_group(&mut self) -> Result<&'source str, (usize, Option<char>)> {
        // If the first character is not `{`, we read a single character.
        let first = self.read_char();
        if first.1 != Some('{') {
            return if first.1.is_some_and(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '|' | '.' | '-' | ',' | '*' | ':')
            }) {
                // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
                Ok(self.input_string.get_unwrap(first.0..self.peek.0))
            } else {
                Err(first)
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
        let closing = self.read_char();
        if closing.1 == Some('}') {
            let end = closing.0;
            // SAFETY: we got `start` and `end` from `CharIndices`, so they are valid bounds.
            Ok(self.input_string.get_unwrap(start..end))
        } else {
            Err(closing)
        }
    }

    pub(crate) fn next_token(&mut self) -> Result<TokLoc<'config>, Box<LatexError<'config>>> {
        let mut is_string_literal = false;
        if let Mode::StringLiteral {
            ref mut arg_num,
            nesting,
        } = self.mode
        {
            // Try subtracting 1 from `arg_num`.
            // If successful, the value must have been > 1.
            if let Some(new_val) = arg_num.checked_sub(1) {
                // We check the nesting here in order to count a `{...}` group as one
                // argument.
                if nesting == self.brace_nesting_level {
                    *arg_num = new_val;
                }
            } else {
                if nesting < self.brace_nesting_level {
                    is_string_literal = true;
                } else {
                    // Finished reading the string literal.
                    self.mode = Mode::default();
                }
            }
        };
        if let Mode::EnvName { is_begin } = self.mode {
            mem::take(&mut self.mode);
            // Read the string literal.
            let result = 'str_literal: {
                // First skip any whitespace.
                self.skip_whitespace();
                let group_loc = self.peek.0;
                // Read the string literal.
                let string_literal = match self.read_ascii_text_group() {
                    Ok(lit) => lit,
                    Err((loc, ch)) => match ch {
                        None => {
                            break 'str_literal Err(LatexError(loc, LatexErrKind::UnexpectedEOF));
                        }
                        Some(ch) => {
                            break 'str_literal Err(LatexError(
                                loc,
                                LatexErrKind::DisallowedChar(ch),
                            ));
                        }
                    },
                };
                // Convert the environment name to the `Env` enum.
                let Some(env) = Env::from_str(string_literal) else {
                    break 'str_literal Err(LatexError(
                        group_loc,
                        LatexErrKind::UnknownEnvironment(string_literal.into()),
                    ));
                };
                if is_begin && env.needs_string_literal() {
                    // Some environments need a string literal after `\begin{...}`.
                    self.mode = Mode::StringLiteral {
                        arg_num: 1,
                        nesting: self.brace_nesting_level,
                    };
                }
                // Return an `EnvName` token.
                Ok(TokLoc(group_loc, Token::EnvName(env)))
            };
            match result {
                Ok(tok) => {
                    return Ok(tok);
                }
                Err(err) => {
                    return Err(Box::new(err));
                }
            }
        }
        let text_mode = matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. });
        if let Some(loc) = self.skip_whitespace()
            && (text_mode || is_string_literal)
        {
            return Ok(TokLoc(loc.get(), Token::Whitespace));
        }

        let (loc, ch) = self.read_char();
        let Some(ch) = ch else {
            return Ok(TokLoc(loc, Token::Eof));
        };
        if ch == '%' {
            // Skip comments.
            while self.peek.1 != Some('\n') && self.peek.1.is_some() {
                self.read_char();
            }
            return self.next_token();
        }
        let tok = match ch {
            '\u{0}' => {
                return Err(Box::new(LatexError(loc, LatexErrKind::DisallowedChar(ch))));
            }
            ' ' => Token::Letter('\u{A0}'),
            '!' => Token::ForceClose(symbol::EXCLAMATION_MARK),
            '#' => {
                if let Some(num) = &mut self.parse_cmd_args
                    && let Some(next) = self.peek.1
                    && next.is_ascii_digit()
                {
                    // In pre-defined commands, `#` is used to denote a parameter.
                    let param_num = (next as u32).wrapping_sub('1' as u32);
                    if !(0..=8).contains(&param_num) {
                        return Err(Box::new(LatexError(
                            loc,
                            LatexErrKind::InvalidParameterNumber,
                        )));
                    }
                    let param_num = param_num as u8;
                    if (param_num + 1) > *num {
                        *num = param_num + 1;
                    }
                    // Discard the digit after `#`.
                    self.read_char();
                    Token::CustomCmdArg(param_num)
                } else {
                    Token::Letter('#')
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
                    Token::BinaryOp(symbol::ASTERISK_OPERATOR)
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
                    return Err(Box::new(LatexError(
                        loc,
                        LatexErrKind::UnexpectedClose(Token::GroupEnd),
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
                let cmd_string = self.read_command();
                if text_mode {
                    // After a command, all whitespace is skipped, even in text mode.
                    // This is done automatically in non-text-mode, but for text
                    // mode we need to do it manually.
                    self.skip_whitespace();
                }
                return self.parse_command(loc, cmd_string);
            }
            c => {
                if c.is_ascii_digit() {
                    Token::Digit(c)
                } else {
                    // Some symbols like '.' and '/' are considered operators by the MathML Core spec,
                    // but in LaTeX they behave like normal identifiers (they are in the "ordinary" class 0).
                    // One might think that they could be rendered as `<mo>` with custom spacing,
                    // but then they still interact with other operators in ways that are not correct.
                    Token::Letter(c)
                }
            }
        };
        if matches!(self.mode, Mode::TextStart) {
            // If we didn't go into `Mode::TextGroup` (by reading a `{`),
            // we go back to math mode after reading one token.
            self.mode = Mode::Math;
        }
        Ok(TokLoc(loc, tok))
    }

    fn parse_command(
        &mut self,
        loc: usize,
        cmd_string: &'source str,
    ) -> Result<TokLoc<'config>, Box<LatexError<'config>>> {
        let tok: Result<Token<'config>, LatexError<'config>> =
            if matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. }) {
                if let Some(tok) = get_text_command(cmd_string) {
                    Ok(tok)
                } else {
                    Err(LatexError(
                        loc,
                        LatexErrKind::UnknownCommand(cmd_string.into()),
                    ))
                }
            } else if let Some(tok) = self
                .custom_cmds
                .and_then(|custom_cmds| custom_cmds.get_command(cmd_string))
                .or_else(|| get_command(cmd_string))
            {
                Ok(tok)
            } else {
                Err(LatexError(
                    loc,
                    LatexErrKind::UnknownCommand(cmd_string.into()),
                ))
            };
        if matches!(self.mode, Mode::TextStart) {
            // If we didn't go into `Mode::TextGroup` (by reading a `{`),
            // we go back to math mode after reading one token.
            self.mode = Mode::Math;
        }
        if let Ok(tok) = &tok {
            if matches!(tok, Token::Text(_)) {
                self.mode = Mode::TextStart;
            } else if matches!(tok, Token::Begin) {
                self.mode = Mode::EnvName { is_begin: true };
            } else if matches!(tok, Token::End) {
                self.mode = Mode::EnvName { is_begin: false };
            } else if let Some(arg_num) = tok.needs_string_literal() {
                self.mode = Mode::StringLiteral {
                    arg_num: arg_num.get(),
                    nesting: self.brace_nesting_level,
                };
            }
        }
        match tok {
            Ok(tok) => Ok(TokLoc(loc, tok)),
            Err(err) => Err(Box::new(err)),
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
    EnvName {
        is_begin: bool, // `true` if it's `\begin`, `false` if it's `\end`.
    },
    StringLiteral {
        /// 1-based index of the argument that is a string literal.
        /// If it is 0, then we are inside the string literal.
        arg_num: u8,
        /// The nesting level of `{` when the string literal was requested.
        nesting: usize,
    },
}

pub(crate) fn recover_limited_ascii(tok: Token) -> Option<char> {
    const COLON: MathMLOperator = symbol::COLON.as_op();
    match tok {
        Token::Letter(ch) if ch.is_ascii_alphabetic() || ch == '.' => Some(ch),
        Token::Whitespace => Some(' '),
        Token::Ord(symbol::VERTICAL_LINE) => Some('|'),
        Token::Punctuation(symbol::COMMA) => Some(','),
        Token::BinaryOp(symbol::MINUS_SIGN) => Some('-'),
        Token::BinaryOp(symbol::ASTERISK_OPERATOR) => Some('*'),
        Token::ForceRelation(COLON) => Some(':'),
        Token::Digit(ch) => Some(ch),
        _ => None,
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
            // Call `lexer.next_token(false)` until we get `Token::EOF`.
            let mut tokens = String::new();
            loop {
                let tokloc = lexer.next_token().unwrap();
                if matches!(tokloc.1, Token::Eof) {
                    break;
                }
                let TokLoc(loc, tok) = tokloc;
                write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
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
                        if matches!(tokloc.1, Token::Eof) {
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
            write!(tokens, "Error at {}: {:?}\n", err.0, err.1).unwrap();
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
            let TokLoc(loc, tok) = tokloc;
            write!(tokens, "{}: {:?}\n", loc, tok).unwrap();
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
            let TokLoc(_, tok) = tokloc;
            if let Some(ch) = recover_limited_ascii(tok) {
                output.push(ch);
            }
            if matches!(tok, Token::Eof) {
                break;
            }
        }
        assert_eq!(input, output);
    }
}
