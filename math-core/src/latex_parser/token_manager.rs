use std::collections::VecDeque;

use crate::LatexError;

use super::{
    lexer::Lexer,
    token::{TokLoc, Token},
};

pub(super) struct TokenManager<'cell, 'source> {
    pub lexer: Lexer<'source, 'source, 'cell>,
    buf: VecDeque<TokLoc<'source>>,
    is_eof: bool,
}

static EOF_TOK: TokLoc = TokLoc(0, Token::Eof);

impl<'cell, 'source> TokenManager<'cell, 'source> {
    pub(super) fn new(lexer: Lexer<'source, 'source, 'cell>) -> Self {
        TokenManager {
            lexer,
            buf: VecDeque::new(),
            is_eof: false,
        }
    }

    /// Ensure that there are at least `n` tokens in the buffer.
    /// If the end of the input is reached, this will stop early.
    pub(super) fn ensure(&mut self, n: usize) -> Result<(), &'cell LatexError<'source>> {
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

    #[inline]
    pub(super) fn peek(&self) -> &TokLoc<'source> {
        // The queue can only be empty if we reached EOF.
        self.buf.front().unwrap_or(&EOF_TOK)
    }

    pub(super) fn peek_two(&mut self) -> Result<&TokLoc<'source>, &'cell LatexError<'source>> {
        self.ensure(2)?;
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.get(1).unwrap_or(&EOF_TOK))
    }

    /// Get the next token from the lexer, replacing the current peek token.
    ///
    /// If there are tokens in the queue, pop the front from the queue instead.
    pub(super) fn next<'arena>(&mut self) -> Result<TokLoc<'source>, &'cell LatexError<'source>> {
        // We ensure two tokens here, so that we can always peek.
        self.ensure(2)?;
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.pop_front().unwrap_or(*&EOF_TOK))
    }

    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<TokLoc<'source>> + Copy]) {
        // Only do anything if the token slice is non-empty.
        // Queue the token stream in reverse order.
        for tok in tokens.iter().rev() {
            self.buf.push_front((*tok).into());
        }
    }
}
