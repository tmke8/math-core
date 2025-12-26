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
    next_non_whitespace: usize,
}

static EOF_TOK: TokLoc = TokLoc(0, Token::Eof);

impl<'source, 'config> TokenManager<'source, 'config> {
    pub(super) fn new(lexer: Lexer<'config, 'source>) -> Result<Self, Box<LatexError<'config>>> {
        let mut tm = TokenManager {
            lexer,
            buf: VecDeque::with_capacity(2),
            lexer_is_eof: false,
            next_non_whitespace: 0,
        };
        // Ensure that we have at least one non-whitespace token in the buffer for peeking.
        let offset = tm.load_token()?;
        tm.next_non_whitespace = offset;
        Ok(tm)
    }

    /// Load the next non-whitespace token from the lexer into the buffer.
    /// If the end of the input is reached, this will return early.
    fn load_token(&mut self) -> Result<usize, Box<LatexError<'config>>> {
        if self.lexer_is_eof {
            // Returning here with offset 0 is the right thing to do,
            // because it will result in an index that is one past the end of the buffer.
            return Ok(0);
        }
        let mut non_whitespace_offset = 0usize;
        loop {
            let tok = self.lexer.next_token()?;
            let is_whitespace = matches!(tok.token(), Token::Whitespace);
            let is_eof = matches!(tok.token(), Token::Eof);
            self.buf.push_back(tok);
            if !is_whitespace {
                break;
            }
            non_whitespace_offset += 1;
            if is_eof {
                self.lexer_is_eof = true;
                // We return with the offset for one past the EOF token.
                // This is needed to ensure that peek() works correctly.
                break;
            }
        }
        Ok(non_whitespace_offset)
    }

    /// Perform a linear search to find the next non-whitespace token in the buffer.
    fn find_next_non_whitespace(&self) -> Option<usize> {
        self.buf
            .iter()
            .position(|tokloc| !matches!(tokloc.token(), Token::Whitespace))
    }

    fn ensure_next_non_whitespace(&mut self) -> Result<(), Box<LatexError<'config>>> {
        let pos = 'pos_calc: {
            // First, try to find the next non-whitespace token in the existing buffer.
            if !self.buf.is_empty()
                && let Some(pos) = self.find_next_non_whitespace()
            {
                break 'pos_calc pos;
            };
            // Then, try to load more tokens until we find one or reach EOF.
            let starting_len = self.buf.len();
            starting_len + self.load_token()?
        };
        self.next_non_whitespace = pos;
        Ok(())
    }

    /// Peek at the next non-whitespace token without consuming it.
    ///
    /// If the lexer has reached the end of the input, this will return an EOF token.
    /// The public interface of `TokenManager` enforces the invariant that there is
    /// always at least one non-whitespace token in the buffer when this is called,
    /// unless EOF has been reached.
    #[inline]
    pub(super) fn peek(&self) -> &TokLoc<'config> {
        // `next_non_whitespace` points to the next non-whitespace token,
        // or to one past the end of the buffer if there is none.
        if let Some(tok) = self.buf.get(self.next_non_whitespace) {
            tok
        } else {
            debug_assert!(self.lexer_is_eof, "peek called without ensure");
            &EOF_TOK
        }
    }

    pub(super) fn peek_second(&mut self) -> Result<&TokLoc<'config>, Box<LatexError<'config>>> {
        match self.find_second_non_whitespace() {
            Some(tok_idx) => Ok(self.buf.get(tok_idx).unwrap_or(&EOF_TOK)),
            None => {
                // Otherwise, load more tokens until we find one or reach EOF.
                let starting_len = self.buf.len();
                let offset = self.load_token()?;
                if let Some(tok) = self.buf.get(starting_len + offset) {
                    Ok(tok)
                } else {
                    debug_assert!(self.lexer_is_eof, "peek_second called without ensure");
                    Ok(&EOF_TOK)
                }
            }
        }
    }

    /// Find the index of the second non-whitespace token in the buffer.
    ///
    /// This function returns an index instead of a reference to the token
    /// in order to avoid issues with the borrow checker.
    fn find_second_non_whitespace(&self) -> Option<usize> {
        // Ensure that the compiler can tell that `self.buf.range(next_non_whitespace..)`
        // cannot panic due to being out of bounds.
        let next_non_whitespace = self.next_non_whitespace;
        if next_non_whitespace < self.buf.len() {
            let mut range = self.buf.range(next_non_whitespace..);
            range.next(); // Skip the first non-whitespace token.
            // If there is a second non-whitespace token in the buffer, return it.
            range.position(|tokloc| !matches!(tokloc.token(), Token::Whitespace))
        } else {
            debug_assert!(self.lexer_is_eof, "peek_second called without ensure");
            Some(self.buf.len())
        }
    }

    /// Get the next non-whitespace token.
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokLoc<'config>, Box<LatexError<'config>>> {
        // Pop elements until we reach `next_non_whitespace`.
        for _ in 0..self.next_non_whitespace {
            let _ = self.buf.pop_front();
        }

        // Now pop the next token.
        if let Some(ret) = self.buf.pop_front() {
            self.ensure_next_non_whitespace()?;
            Ok(ret)
        } else {
            // We must have reached EOF previously.
            debug_assert!(self.lexer_is_eof, "next called without ensure");
            Ok(EOF_TOK)
        }
    }

    /// Get the next token which may be whitespace.
    pub(super) fn next_with_whitespace(
        &mut self,
    ) -> Result<TokLoc<'config>, Box<LatexError<'config>>> {
        if let Some(ret) = self.buf.pop_front() {
            // `next_non_whitespace` may need to be updated.
            if let Some(new_pos) = self.next_non_whitespace.checked_sub(1) {
                self.next_non_whitespace = new_pos;
            } else {
                // We popped `next_non_whitespace` itself, so we need to find the next one.
                self.ensure_next_non_whitespace()?;
            }
            Ok(ret)
        } else {
            // We must have reached EOF previously.
            debug_assert!(
                self.lexer_is_eof,
                "next_with_whitespace called without ensure"
            );
            Ok(EOF_TOK)
        }
    }

    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<TokLoc<'config>> + Copy]) {
        self.buf.reserve(tokens.len());
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            self.buf.push_front((*tok).into());
        }

        // Update the next_non_whitespace position.
        if let Some(pos) = self.find_next_non_whitespace() {
            self.next_non_whitespace = pos;
        } else {
            // There is only one scenario in which we wouldn't find a non-whitespace token:
            // We reached EOF previously and all queued tokens are whitespace.
            debug_assert!(self.lexer_is_eof, "queue_in_front called without ensure");
            self.next_non_whitespace = self.buf.len();
        }
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    pub(super) fn read_group(
        &mut self,
        tokens: &mut Vec<TokLoc<'config>>,
        with_whitespace: bool,
    ) -> Result<(), Box<LatexError<'config>>> {
        let mut nesting_level = 0usize;
        loop {
            let tokloc = if with_whitespace {
                self.next_with_whitespace()
            } else {
                self.next()
            };
            let TokLoc(loc, tok) = tokloc?;
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
            let tokens = match manager.read_group(&mut tokens, false) {
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
    fn test_get_whitespace_tokens() {
        let input = r"\text{  x +   y }";
        // let input = r"\text  xy";
        let lexer = Lexer::new(input, false, None);
        let mut manager = TokenManager::new(lexer).expect("Failed to create TokenManager");

        let mut token_str = String::new();

        loop {
            let TokLoc(loc, tok) = manager.next_with_whitespace().unwrap();
            if matches!(tok, Token::Eof) {
                break;
            }
            write!(token_str, "{}: {:?}\n", loc, tok).unwrap();
        }

        assert_snapshot!("next_with_whitespace", &token_str, input);
    }
}
