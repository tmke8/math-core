use std::mem;

use crate::mathml_renderer::arena::Arena;

use super::{
    lexer::Lexer,
    token::{TokResult, Token},
};

pub(super) struct TokenManager<'arena, 'source> {
    pub lexer: Lexer<'source, 'source>,
    pub peek: TokResult<'arena, 'source>,
    stack: Vec<TokResult<'arena, 'source>>,
}

impl<'arena, 'source> TokenManager<'arena, 'source> {
    pub(super) fn new(lexer: Lexer<'source, 'source>, initial_peek: Token<'source>) -> Self {
        TokenManager {
            lexer,
            peek: TokResult(0, Ok(initial_peek)),
            stack: Vec::new(),
        }
    }

    /// Get the next token from the lexer, replacing the current peek token.
    ///
    /// If there are tokens on the stack, pop the top token from the stack instead.
    pub(super) fn next(&mut self, arena: &'arena Arena) -> TokResult<'arena, 'source> {
        let peek_token = if let Some(tok) = self.stack.pop() {
            tok
        } else {
            match self.lexer.next_token() {
                Ok((loc, tok)) => TokResult(loc, Ok(tok)),
                Err(e) => {
                    let err = arena.alloc(e.1);
                    TokResult(e.0, Err(err))
                }
            }
        };
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek, peek_token)
    }

    pub(super) fn add_to_stack(&mut self, tokens: &[impl Into<TokResult<'arena, 'source>> + Copy]) {
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

    #[inline]
    pub(super) fn is_empty_stack(&self) -> bool {
        self.stack.is_empty()
    }
}
