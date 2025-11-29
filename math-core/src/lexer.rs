use std::cell::OnceCell;
use std::mem;
use std::num::{NonZeroU8, NonZeroUsize};
use std::str::CharIndices;

use mathml_renderer::symbol;

use crate::CustomCmds;
use crate::commands::{get_command, get_text_command};
use crate::environments::Env;
use crate::error::{GetUnwrap, LatexErrKind, LatexError};
use crate::token::{TokLoc, Token};

/// Lexer
pub(crate) struct Lexer<'config, 'source, 'cell>
where
    'config: 'source,
    'source: 'cell,
{
    input: CharIndices<'source>,
    peek: (usize, Option<char>),
    input_string: &'source str,
    input_length: usize,
    mode: Mode,
    brace_nesting_level: usize,
    parse_cmd_args: Option<u8>,
    custom_cmds: Option<&'config CustomCmds>,
    error_slot: &'cell OnceCell<LatexError<'source>>,
    string_storage: &'cell mut String,
}

impl<'config, 'source, 'cell> Lexer<'config, 'source, 'cell> {
    /// Receive the input source code and generate a LEXER instance.
    pub(crate) fn new(
        input: &'source str,
        parsing_custom_cmds: bool,
        custom_cmds: Option<&'config CustomCmds>,
        error_slot: &'cell OnceCell<LatexError<'source>>,
        string_storage: &'cell mut String,
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
            error_slot,
            string_storage,
        };
        lexer.read_char(); // Initialize `peek`.
        lexer
    }

    #[inline]
    pub(super) fn get_str(&self, start: usize, end: usize) -> Option<&'config str> {
        self.custom_cmds
            .and_then(|cmds| cmds.get_string_literal(start, end))
    }

    #[inline]
    pub(super) fn alloc_err(&mut self, err: LatexError<'source>) -> &'cell LatexError<'source> {
        debug_assert!(
            self.error_slot.get().is_none(),
            "A previous error was already allocated and not returned"
        );
        self.error_slot.get_or_init(|| err)
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

    /// Read a group of tokens, ending with (an unopened) `}`.
    pub(super) fn read_group(
        &mut self,
        tokens: &mut Vec<TokLoc<'source>>,
    ) -> Result<(), &'cell LatexError<'source>> {
        let start_nesting_level = self.brace_nesting_level;
        loop {
            let TokLoc(loc, tok) = self.next_token()?;
            match tok {
                Token::GroupEnd => {
                    // If the nesting level reaches one below where we started, we
                    // stop reading.
                    if self.brace_nesting_level + 1 == start_nesting_level {
                        // We break directly without pushing the `}` token.
                        break;
                    }
                }
                Token::Eof => {
                    return Err(self.alloc_err(LatexError(loc, LatexErrKind::UnclosedGroup(tok))));
                }
                _ => {}
            }
            tokens.push(TokLoc(loc, tok));
        }
        Ok(())
    }

    /// Generate the next token.
    pub(crate) fn next_token(&mut self) -> Result<TokLoc<'source>, &'cell LatexError<'source>> {
        // Put the string literal in a token.
        match self.next_token_or_string_literal()? {
            LexResult::StringLiteral(loc, s) => Ok(TokLoc(loc, Token::StringLiteral(s))),
            LexResult::Token(tokloc) => Ok(tokloc),
        }
    }

    /// Generate the next token, without references to the source string.
    pub(crate) fn next_static_token(
        &mut self,
    ) -> Result<TokLoc<'config>, &'cell LatexError<'source>> {
        // Put the string literal in a token.
        match self.next_token_or_string_literal()? {
            LexResult::StringLiteral(loc, s) => {
                let start = self.string_storage.len();
                self.string_storage.push_str(s);
                let end = self.string_storage.len();
                Ok(TokLoc(loc, Token::StoredStringLiteral(start, end)))
            }
            LexResult::Token(tokloc) => Ok(tokloc),
        }
    }

    fn next_token_or_string_literal(
        &mut self,
    ) -> Result<LexResult<'config, 'source>, &'cell LatexError<'source>> {
        let mut is_string_literal = false;
        if let Mode::StringLiteral {
            ref mut arg_num,
            nesting,
        } = self.mode
            // We check the nesting here in order to count a `{...}` group as one
            // argument.
            && nesting == self.brace_nesting_level
        {
            // Try subtracting 1 from `arg_num`.
            let new_val = NonZeroU8::new(arg_num.get() - 1);
            if let Some(new_val) = new_val {
                // If successful, the value must have been > 1.
                *arg_num = new_val;
            } else {
                is_string_literal = true;
            }
        };
        if matches!(self.mode, Mode::EnvName { .. }) || is_string_literal {
            let mode = mem::take(&mut self.mode);
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
                if let Mode::EnvName { is_begin } = mode {
                    // Convert the environment name to the `Env` enum.
                    let Some(env) = Env::from_str(string_literal) else {
                        break 'str_literal Err(LatexError(
                            group_loc,
                            LatexErrKind::UnknownEnvironment(string_literal),
                        ));
                    };
                    if is_begin && env.needs_string_literal() {
                        // Some environments need a string literal after `\begin{...}`.
                        const ONE: NonZeroU8 = NonZeroU8::new(1).unwrap();
                        self.mode = Mode::StringLiteral {
                            arg_num: ONE,
                            nesting: self.brace_nesting_level,
                        };
                    }
                    // Return an `EnvName` token.
                    Ok(LexResult::Token(TokLoc(group_loc, Token::EnvName(env))))
                } else {
                    Ok(LexResult::StringLiteral(group_loc, string_literal))
                }
            };
            match result {
                Ok(tok) => {
                    return Ok(tok);
                }
                Err(err) => {
                    return Err(self.alloc_err(err));
                }
            }
        }
        let text_mode = matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. });
        if let Some(loc) = self.skip_whitespace()
            && text_mode
        {
            return Ok(LexResult::Token(TokLoc(loc.get(), Token::Whitespace)));
        }

        let (loc, ch) = self.read_char();
        let Some(ch) = ch else {
            return Ok(LexResult::Token(TokLoc(loc, Token::Eof)));
        };
        if ch == '%' {
            // Skip comments.
            while self.peek.1 != Some('\n') && self.peek.1.is_some() {
                self.read_char();
            }
            return self.next_token_or_string_literal();
        }
        let tok = match ch {
            '\u{0}' => {
                return Err(self.alloc_err(LatexError(loc, LatexErrKind::DisallowedChar(ch))));
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
                        return Err(
                            self.alloc_err(LatexError(loc, LatexErrKind::InvalidParameterNumber))
                        );
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
                    return Err(self.alloc_err(LatexError(
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
                return self.parse_command(loc, cmd_string).map(LexResult::Token);
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
        Ok(LexResult::Token(TokLoc(loc, tok)))
    }

    fn parse_command(
        &mut self,
        loc: usize,
        cmd_string: &'source str,
    ) -> Result<TokLoc<'config>, &'cell LatexError<'source>> {
        let tok: Result<Token<'config>, LatexError<'source>> =
            if matches!(self.mode, Mode::TextStart | Mode::TextGroup { .. }) {
                if let Some(tok) = get_text_command(cmd_string) {
                    Ok(tok)
                } else {
                    Err(LatexError(loc, LatexErrKind::UnknownCommand(cmd_string)))
                }
            } else if let Some(tok) = self
                .custom_cmds
                .and_then(|custom_cmds| custom_cmds.get_command(cmd_string))
                .or_else(|| get_command(cmd_string))
            {
                Ok(tok)
            } else {
                Err(LatexError(loc, LatexErrKind::UnknownCommand(cmd_string)))
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
                    arg_num,
                    nesting: self.brace_nesting_level,
                };
            }
        }
        match tok {
            Ok(tok) => Ok(TokLoc(loc, tok)),
            Err(err) => Err(self.alloc_err(err)),
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
        arg_num: NonZeroU8,
        /// The nesting level of `{` when the string literal was requested.
        nesting: usize,
    },
}

#[derive(Debug)]
enum LexResult<'config, 'source> {
    Token(TokLoc<'config>),
    StringLiteral(usize, &'source str), // The string and its starting location.
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
            ("color", r"{x\color{red} y}"),
            ("color_whitespace", r"{x\color     {red} y}"),
            ("color_newline", "{x\\color\n{red} y}"),
            ("genfrac_with_parens", r"\genfrac(]{0pt}{2}{a+b}{c+d}"),
            (
                "genfrac_with_one_sided_parens",
                r"\genfrac{}]{0pt}{2}{a+b}{c+d}",
            ),
            ("genfrac_without_parens", r"\genfrac{}{}{0pt}{2}{a+b}{c+d}"),
            ("begin_array", r"\begin{array}{c|c}"),
            ("end_array", r"\end{array}{c|c}"),
        ];

        let string_storage = &mut String::new();
        for (name, problem) in problems.into_iter() {
            let error_slot = OnceCell::new();
            let mut lexer = Lexer::new(problem, false, None, &error_slot, string_storage);
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
        let string_storage = &mut String::new();
        for (name, problem) in problems.into_iter() {
            let error_slot = OnceCell::new();
            let mut lexer = Lexer::new(problem, false, None, &error_slot, string_storage);
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

        let string_storage = &mut String::new();
        for (name, problem) in problems.into_iter() {
            let error_slot = OnceCell::new();
            let mut lexer = Lexer::new(problem, false, None, &error_slot, string_storage);
            // Check that the first token is `GroupBegin`.
            assert!(matches!(lexer.next_token().unwrap().1, Token::GroupBegin));
            let mut tokens = Vec::new();
            let tokens = match lexer.read_group(&mut tokens) {
                Ok(()) => {
                    let mut token_str = String::new();
                    for TokLoc(loc, tok) in tokens {
                        write!(token_str, "{}: {:?}\n", loc, tok).unwrap();
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
        let error_slot = OnceCell::new();
        let mut string_storage = String::new();
        let mut lexer = Lexer::new(
            problem,
            parsing_custom_cmds,
            None,
            &error_slot,
            &mut string_storage,
        );
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
}
