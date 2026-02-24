use std::collections::VecDeque;

use crate::{
    error::{LatexErrKind, LatexError},
    lexer::Lexer,
    token::{EndToken, Span, TokSpan, Token},
};

/// A token queue that allows peeking at the next non-whitespace token.
pub(super) struct TokenQueue<'source, 'config> {
    pub lexer: Lexer<'config, 'source>,
    queue: VecDeque<TokSpan<'source>>,
    lexer_is_eoi: bool,
    next_non_whitespace: usize,
}

static EOI_TOK: TokSpan = TokSpan::new(Token::Eoi, Span(0, 0));

impl<'source, 'config> TokenQueue<'source, 'config> {
    pub(super) fn new(lexer: Lexer<'config, 'source>) -> Result<Self, Box<LatexError>> {
        let mut tm = TokenQueue {
            lexer,
            queue: VecDeque::with_capacity(2),
            lexer_is_eoi: false,
            next_non_whitespace: 0,
        };
        // Ensure that we have at least one non-whitespace token in the buffer for peeking.
        let offset = tm.load_token(SkipMode::Whitespace)?;
        tm.next_non_whitespace = offset;
        Ok(tm)
    }

    /// Load the next not-skipped token from the lexer into the buffer.
    /// If the end of the input is reached, this will return early.
    fn load_token(&mut self, skip_mode: SkipMode) -> Result<usize, Box<LatexError>> {
        if self.lexer_is_eoi {
            // Returning here with offset 0 is the right thing to do,
            // because it will result in an index that is one past the end of the buffer.
            return Ok(0);
        }
        let mut non_skipped_offset = 0usize;
        let predicate = match skip_mode {
            SkipMode::Whitespace => is_not_whitespace,
            SkipMode::NoClass => has_class,
        };
        loop {
            let tok = self.lexer.next_token()?;
            let not_skipped = predicate(&tok);
            let is_eoi = matches!(tok.token(), Token::Eoi);
            self.queue.push_back(tok);
            if not_skipped {
                break;
            }
            non_skipped_offset += 1;
            if is_eoi {
                self.lexer_is_eoi = true;
                // We return with the offset for one past the EOI token.
                // This is needed to ensure that peek() works correctly.
                break;
            }
        }
        Ok(non_skipped_offset)
    }

    /// Perform a linear search to find the next non-whitespace token in the buffer.
    fn find_next_non_whitespace(&self) -> Option<usize> {
        self.queue.iter().position(is_not_whitespace)
    }

    fn ensure_next_non_whitespace(&mut self) -> Result<(), Box<LatexError>> {
        let pos = 'pos_calc: {
            // First, try to find the next non-whitespace token in the existing buffer.
            if !self.queue.is_empty()
                && let Some(pos) = self.find_next_non_whitespace()
            {
                break 'pos_calc pos;
            };
            // Then, try to load more tokens until we find one or reach EOI.
            let starting_len = self.queue.len();
            starting_len + self.load_token(SkipMode::Whitespace)?
        };
        self.next_non_whitespace = pos;
        Ok(())
    }

    /// Peek at the next non-whitespace token without consuming it.
    ///
    /// If the lexer has reached the end of the input, this will return an EOI token.
    /// The public interface of `TokenManager` enforces the invariant that there is
    /// always at least one non-whitespace token in the buffer when this is called,
    /// unless EOI has been reached.
    #[inline]
    pub(super) fn peek(&self) -> &TokSpan<'source> {
        // `next_non_whitespace` points to the next non-whitespace token,
        // or to one past the end of the buffer if there is none.
        if let Some(tok) = self.queue.get(self.next_non_whitespace) {
            tok
        } else {
            debug_assert!(self.lexer_is_eoi, "peek called without ensure");
            &EOI_TOK
        }
    }

    /// Find or load a token which is not skipped according to `skip_mode`.
    ///
    /// This function starts its search after `next_non_whitespace` (i.e., it skips
    /// the first non-whitespace token). The idea is that the caller has already
    /// checked `next_non_whitespace` or is not interested in it.
    ///
    /// This function returns an index instead of a reference to the token
    /// in order to avoid issues with the borrow checker.
    fn find_or_load_after_next(
        &mut self,
        skip_mode: SkipMode,
    ) -> Result<&TokSpan<'source>, Box<LatexError>> {
        // We use a block here which returns an index to avoid borrow checker issues.
        let tok_idx = {
            // Ensure that the compiler can tell that `self.queue.range(start..)`
            // cannot panic due to being out of bounds.
            let start = self.next_non_whitespace;
            if start < self.queue.len() {
                let mut range = self.queue.range(start..);
                range.next(); // Skip `next_non_whitespace`.
                let predicate = match skip_mode {
                    SkipMode::Whitespace => is_not_whitespace,
                    SkipMode::NoClass => has_class,
                };
                range.position(predicate).map(|pos| start + 1 + pos)
            } else {
                debug_assert!(
                    self.lexer_is_eoi,
                    "find_or_load_after_next called without ensure"
                );
                Some(self.queue.len())
            }
        };

        match tok_idx {
            Some(tok_idx) => Ok(self.queue.get(tok_idx).unwrap_or(&EOI_TOK)),
            None => {
                // Otherwise, load more tokens until we find one or reach EOI.
                let starting_len = self.queue.len();
                let offset = self.load_token(skip_mode)?;
                if let Some(tok) = self.queue.get(starting_len + offset) {
                    Ok(tok)
                } else {
                    debug_assert!(
                        self.lexer_is_eoi,
                        "find_or_load_after_next called without ensure"
                    );
                    Ok(&EOI_TOK)
                }
            }
        }
    }

    pub(super) fn peek_second(&mut self) -> Result<&TokSpan<'source>, Box<LatexError>> {
        self.find_or_load_after_next(SkipMode::Whitespace)
    }

    /// Peek at the first token which has a character class.
    ///
    /// This excludes, for example, `Space` tokens.
    pub(super) fn peek_class_token(&mut self) -> Result<&TokSpan<'source>, Box<LatexError>> {
        // First check the common case where the next token is already a token with class.
        if has_class(self.peek()) {
            return Ok(self.peek());
        }
        self.find_or_load_after_next(SkipMode::NoClass)
    }

    /// Get the next non-whitespace token.
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokSpan<'source>, Box<LatexError>> {
        // Pop elements until we reach `next_non_whitespace`.
        for _ in 0..self.next_non_whitespace {
            let _ = self.queue.pop_front();
        }

        // Now pop the next token.
        if let Some(ret) = self.queue.pop_front() {
            self.ensure_next_non_whitespace()?;
            Ok(ret)
        } else {
            // We must have reached EOI previously.
            debug_assert!(self.lexer_is_eoi, "next called without ensure");
            Ok(EOI_TOK)
        }
    }

    /// Get the next token which may be whitespace.
    pub(super) fn next_with_whitespace(&mut self) -> Result<TokSpan<'source>, Box<LatexError>> {
        if let Some(ret) = self.queue.pop_front() {
            // `next_non_whitespace` may need to be updated.
            if let Some(new_pos) = self.next_non_whitespace.checked_sub(1) {
                self.next_non_whitespace = new_pos;
            } else {
                // We popped `next_non_whitespace` itself, so we need to find the next one.
                self.ensure_next_non_whitespace()?;
            }
            Ok(ret)
        } else {
            // We must have reached EOI previously.
            debug_assert!(
                self.lexer_is_eoi,
                "next_with_whitespace called without ensure"
            );
            Ok(EOI_TOK)
        }
    }

    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<TokSpan<'source>> + Copy]) {
        self.queue.reserve(tokens.len());
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            self.queue.push_front((*tok).into());
        }

        // Update the next_non_whitespace position.
        if let Some(pos) = self.find_next_non_whitespace() {
            self.next_non_whitespace = pos;
        } else {
            // There is only one scenario in which we wouldn't find a non-whitespace token:
            // We reached EOI previously and all queued tokens are whitespace.
            debug_assert!(self.lexer_is_eoi, "queue_in_front called without ensure");
            self.next_non_whitespace = self.queue.len();
        }
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    ///
    /// The initial `{` must have already been consumed. The closing `}` is not included
    /// in the output token vector.
    pub(super) fn read_group(
        &mut self,
        tokens: &mut Vec<TokSpan<'source>>,
    ) -> Result<usize, Box<LatexError>> {
        let mut nesting_level = 0usize;
        let end = loop {
            let tokloc = self.next_with_whitespace()?;
            match tokloc.token() {
                Token::GroupBegin => {
                    nesting_level += 1;
                }
                Token::GroupEnd => {
                    // If the nesting level reaches one below where we started, we
                    // stop reading.
                    let Some(new_level) = nesting_level.checked_sub(1) else {
                        // We break directly without pushing the `}` token.
                        break tokloc.span().end();
                    };
                    nesting_level = new_level;
                }
                Token::Eoi => {
                    return Err(Box::new(LatexError(
                        tokloc.span().into(),
                        LatexErrKind::UnclosedGroup(EndToken::GroupClose),
                    )));
                }
                _ => {}
            }
            tokens.push(tokloc);
        };
        Ok(end)
    }
}

fn is_not_whitespace(tok: &TokSpan) -> bool {
    !matches!(tok.token(), Token::Whitespace)
}

fn has_class(tok: &TokSpan) -> bool {
    !matches!(
        tok.token(),
        Token::Whitespace | Token::Space(_) | Token::Not
    )
}

enum SkipMode {
    Whitespace,
    NoClass,
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
            let mut manager = TokenQueue::new(lexer).expect("Failed to create TokenManager");
            // Load up some tokens to ensure the code can deal with that.
            manager.load_token(SkipMode::Whitespace).unwrap();
            manager.load_token(SkipMode::Whitespace).unwrap();
            // Check that the first token is `GroupBegin`.
            assert!(matches!(manager.next().unwrap().token(), Token::GroupBegin));
            let mut tokens = Vec::new();
            let tokens = match manager.read_group(&mut tokens) {
                Ok(_) => {
                    let mut token_str = String::new();
                    for tokloc in tokens {
                        let (tok, span) = tokloc.into_parts();
                        write!(token_str, "{}..{}: {:?}\n", span.start(), span.end(), tok).unwrap();
                    }
                    token_str
                }
                Err(err) => format!("Error at {}..{}: {:?}", err.0.start, err.0.end, err.1),
            };
            assert_snapshot!(name, &tokens, problem);
        }
    }

    #[test]
    fn test_get_whitespace_tokens() {
        let input = r"\text{  x +   y }";
        // let input = r"\text  xy";
        let lexer = Lexer::new(input, false, None);
        let mut manager = TokenQueue::new(lexer).expect("Failed to create TokenManager");

        let mut token_str = String::new();

        loop {
            let (tok, span) = manager.next_with_whitespace().unwrap().into_parts();
            if matches!(tok, Token::Eoi) {
                break;
            }
            write!(token_str, "{}..{}: {:?}\n", span.start(), span.end(), tok).unwrap();
        }

        assert_snapshot!("next_with_whitespace", &token_str, input);
    }

    #[test]
    fn test_find_or_load_after_next() {
        let input = r"x y z";
        // let input = r"\text  xy";
        let lexer = Lexer::new(input, false, None);
        let mut queue = TokenQueue::new(lexer).expect("Failed to create TokenManager");
        queue.next().unwrap(); // Consume 'x'
        assert_eq!(queue.next_non_whitespace, 1);
        assert_eq!(queue.queue.len(), 2);
        assert!(matches!(queue.queue[0].token(), Token::Whitespace));
        assert!(matches!(queue.peek().token(), Token::Letter('y')));

        // Test the branch that needs to load more tokens.
        let tok = queue.find_or_load_after_next(SkipMode::Whitespace).unwrap();
        assert!(matches!(tok.token(), Token::Letter('z')));
        assert_eq!(queue.queue.len(), 4);
        assert!(matches!(queue.queue[0].token(), Token::Whitespace));
        assert!(matches!(queue.queue[2].token(), Token::Whitespace));

        // Test the branch that finds the token in the existing buffer.
        let tok = queue.find_or_load_after_next(SkipMode::Whitespace).unwrap();
        assert!(matches!(tok.token(), Token::Letter('z')));
        assert_eq!(queue.queue.len(), 4);
        assert!(matches!(queue.queue[0].token(), Token::Whitespace));
        assert!(matches!(queue.queue[2].token(), Token::Whitespace));
    }
}
