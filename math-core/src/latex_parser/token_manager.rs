use std::collections::VecDeque;

use super::{
    error::{LatexErrKind, LatexError},
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
    pub(super) fn new(
        lexer: Lexer<'source, 'source, 'cell>,
    ) -> Result<Self, &'cell LatexError<'source>> {
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

    pub(super) fn peek_second(&mut self) -> Result<&TokLoc<'source>, &'cell LatexError<'source>> {
        self.ensure(2)?;
        // The queue can only be empty if we reached EOF.
        Ok(self.buf.get(1).unwrap_or(&EOF_TOK))
    }

    /// Get the next token.
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokLoc<'source>, &'cell LatexError<'source>> {
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

    pub(super) fn parse_string_literal(
        &mut self,
    ) -> Result<(usize, &'source str), &'cell LatexError<'source>> {
        let TokLoc(loc, string) = self.next()?;
        let string = match string {
            Token::StringLiteral(s) => Some(s),
            Token::StoredStringLiteral(start, end) => self.lexer.get_str(start, end),
            _ => None,
        };
        if let Some(string) = string {
            Ok((loc, string))
        } else {
            Err(self
                .lexer
                .alloc_err(LatexError(loc, LatexErrKind::Internal)))
        }
    }
}
