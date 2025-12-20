use std::collections::VecDeque;

use crate::{
    error::{LatexErrKind, LatexError},
    lexer::Lexer,
    token::{TokLoc, Token},
};

pub(super) struct TokenManager<'source> {
    pub lexer: Lexer<'source, 'source>,
    buf: VecDeque<TokLoc<'source>>,
    is_eof: bool,
}

static EOF_TOK: TokLoc = TokLoc(0, Token::Eof);

impl<'source> TokenManager<'source> {
    pub(super) fn new(lexer: Lexer<'source, 'source>) -> Result<Self, Box<LatexError<'source>>> {
        let mut tm = TokenManager {
            lexer,
            buf: VecDeque::new(),
            is_eof: false,
        };
        // Ensure that we have at least one token in the buffer for peeking.
        tm.ensure(1)?;
        Ok(tm)
    }

    /// Ensure that there are at least `n` tokens in the buffer.
    /// If the end of the input is reached, this will stop early.
    pub(super) fn ensure(&mut self, n: usize) -> Result<(), Box<LatexError<'source>>> {
        if self.is_eof {
            return Ok(());
        }
        while self.buf.len() < n {
            let tok = self.lexer.next_token()?;
            let is_eof = matches!(tok.1, Token::Eof);
            self.buf.push_back(tok);
            if is_eof {
                self.is_eof = true;
                break;
            }
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
    pub(super) fn peek(&self) -> &TokLoc<'source> {
        if !self.is_eof {
            debug_assert!(!self.buf.is_empty(), "peek called without ensure");
        }
        // The queue can only be empty if we reached EOF.
        self.buf.front().unwrap_or(&EOF_TOK)
    }

    pub(super) fn peek_second(&mut self) -> Result<&TokLoc<'source>, Box<LatexError<'source>>> {
        self.ensure(2)?;
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.get(1).unwrap_or(&EOF_TOK))
    }

    /// Get the next token.
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokLoc<'source>, Box<LatexError<'source>>> {
        // We ensure two tokens here, so that we can always peek.
        self.ensure(2)?;
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.pop_front().unwrap_or(EOF_TOK))
    }

    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<TokLoc<'source>> + Copy]) {
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            self.buf.push_front((*tok).into());
        }
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    pub(super) fn read_group(
        &mut self,
        tokens: &mut Vec<TokLoc<'source>>,
    ) -> Result<(), Box<LatexError<'source>>> {
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
            manager.ensure(3).unwrap();
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
