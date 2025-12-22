use std::collections::VecDeque;

use crate::{
    error::{LatexErrKind, LatexError},
    lexer::Lexer,
    token::{TokLoc, Token},
};

pub(super) struct TokenManager<'source, 'config> {
    pub lexer: Lexer<'config, 'source>,
    buf: VecDeque<TokLoc<'config>>,
    lexer_is_eof: bool,
}

static EOF_TOK: TokLoc = TokLoc(0, Token::Eof);

impl<'source, 'config> TokenManager<'source, 'config> {
    pub(super) fn new(lexer: Lexer<'config, 'source>) -> Result<Self, Box<LatexError<'config>>> {
        let mut tm = TokenManager {
            lexer,
            buf: VecDeque::with_capacity(2),
            lexer_is_eof: false,
        };
        // Ensure that we have at least one token in the buffer for peeking.
        tm.load_token()?;
        Ok(tm)
    }

    /// Load the next token from the lexer into the buffer.
    /// If the end of the input is reached, this will return early.
    fn load_token(&mut self) -> Result<(), Box<LatexError<'config>>> {
        if self.lexer_is_eof {
            return Ok(());
        }
        let tok = self.lexer.next_token()?;
        let lexer_is_eof = matches!(tok.token(), Token::Eof);
        self.buf.push_back(tok);
        if lexer_is_eof {
            self.lexer_is_eof = true;
        }
        Ok(())
    }

    /// Peek at the next token without consuming it.
    ///
    /// If the lexer has reached the end of the input, this will return an EOF token.
    /// The public interface of `TokenManager` enforces the invariant that there is
    /// always at least one token in the buffer when this is called, unless EOF has
    /// been reached.
    #[inline]
    pub(super) fn peek(&self) -> &TokLoc<'config> {
        if !self.lexer_is_eof {
            debug_assert!(!self.buf.is_empty(), "peek called without ensure");
        }
        // The queue can only be empty if we reached EOF.
        self.buf.front().unwrap_or(&EOF_TOK)
    }

    pub(super) fn peek_second(&mut self) -> Result<&TokLoc<'config>, Box<LatexError<'config>>> {
        for _ in self.buf.len()..2 {
            self.load_token()?;
        }
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.get(1).unwrap_or(&EOF_TOK))
    }

    /// Get the next token.
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokLoc<'config>, Box<LatexError<'config>>> {
        // The queue can only be empty if we reached EOF.
        if let Some(ret) = self.buf.pop_front() {
            // We ensure a token in the queue here, so that we can always peek.
            if self.buf.is_empty() {
                self.load_token()?;
            }
            Ok(ret)
        } else {
            // We must have reached EOF previously.
            Ok(EOF_TOK)
        }
    }

    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<TokLoc<'config>> + Copy]) {
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            self.buf.push_front((*tok).into());
        }
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    pub(super) fn read_group(
        &mut self,
        tokens: &mut Vec<TokLoc<'config>>,
    ) -> Result<(), Box<LatexError<'config>>> {
        let mut nesting_level = 0usize;
        loop {
            let TokLoc(loc, tok) = self.next()?;
            match tok {
                Token::GroupBegin => {
                    nesting_level += 1;
                }
                Token::GroupEnd => {
                    // If the nesting level reaches one below where we started, we
                    // stop reading.
                    let Some(new_level) = nesting_level.checked_sub(1) else {
                        // We break directly without pushing the `}` token.
                        break;
                    };
                    nesting_level = new_level;
                }
                Token::Eof => {
                    return Err(Box::new(LatexError(
                        loc,
                        LatexErrKind::UnclosedGroup(Token::GroupEnd),
                    )));
                }
                _ => {}
            }
            tokens.push(TokLoc(loc, tok));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use insta::assert_snapshot;

    use super::*;

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
            let lexer = Lexer::new(problem, false, None);
            let mut manager = TokenManager::new(lexer).expect("Failed to create TokenManager");
            // Load up some tokens to ensure the code can deal with that.
            manager.load_token().unwrap();
            manager.load_token().unwrap();
            // Check that the first token is `GroupBegin`.
            assert!(matches!(manager.next().unwrap().1, Token::GroupBegin));
            let mut tokens = Vec::new();
            let tokens = match manager.read_group(&mut tokens) {
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
}
