use std::mem;

use crate::LatexError;

use super::{
    lexer::Lexer,
    token::{TokLoc, Token},
};

pub(super) struct TokenManager<'cell, 'source> {
    pub lexer: Lexer<'source, 'source, 'cell>,
    pub peek: TokLoc<'source>,
    stack: Vec<TokLoc<'source>>,
}

impl<'cell, 'source> TokenManager<'cell, 'source> {
    pub(super) fn new(lexer: Lexer<'source, 'source, 'cell>, initial_peek: Token<'source>) -> Self {
        TokenManager {
            lexer,
            peek: TokLoc(0, initial_peek),
            stack: Vec::new(),
        }
    }

    /// Get the next token from the lexer, replacing the current peek token.
    ///
    /// If there are tokens on the stack, pop the top token from the stack instead.
    pub(super) fn next<'arena>(&mut self) -> Result<TokLoc<'source>, &'cell LatexError<'source>> {
        let peek_token = if let Some(tok) = self.stack.pop() {
            tok
        } else {
            self.lexer.next_token()?
        };
        // Return the previous peek token and store the new peek token.
        Ok(mem::replace(&mut self.peek, peek_token))
    }

    pub(super) fn add_to_stack(&mut self, tokens: &[impl Into<TokLoc<'source>> + Copy]) {
        // Only do something if the token slice is non-empty.
        if let [head, tail @ ..] = tokens {
            // Replace the peek token with the first token of the token stream.
            let old_peek = mem::replace(&mut self.peek, (*head).into());
            // Put the old peek token onto the token stack.
            self.stack.push(old_peek);
            // Put the rest of the token stream onto the token stack in reverse order.
            for tok in tail.iter().rev() {
                self.stack.push((*tok).into());
            }
        }
    }
}
