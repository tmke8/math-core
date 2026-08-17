use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::ops::Range;

use crate::{
    ParserConfig,
    character_class::Class,
    commands::resolve_builtin_cmd,
    custom_cmds::{CmdSource, CustomCmds},
    error::{LatexErrKind, LatexError},
    global_state::GlobalState,
    lexer::{Lexer, LexerOutput},
    string_pool::InternedStr,
    token::{EndToken, Span, TokSpan, Token},
};

/// A token queue that allows peeking at the next non-whitespace token.
///
/// This is also where command names are given their meaning: the lexer hands out a
/// [`LexerOutput::CommandName`] for every command it reads, and it is resolved here, both when
/// it comes out of the lexer and when it comes out of the body of a custom command. The latter
/// is what makes it possible for a body to mention a command which is only defined later.
///
/// The queue holds on to the name a command came from, next to the meaning it was given; see
/// [`QueuedTok`].
pub(super) struct TokenQueue<'state, 'arena> {
    lexer: Lexer<'arena>,
    pub stores: Stores<'state, 'arena>,
    queue: VecDeque<QueuedTok<'arena>>,
    lexer_is_eoi: bool,
    next_non_whitespace: usize,
}

/// A token in the queue, together with the name of the command it came from.
///
/// Keeping the name is what lets the body of a `\newcommand` be recorded by name rather than by
/// the meaning those names happen to have while the body is being read; the meaning is only
/// looked up when the command is expanded, as it is in LaTeX. The token itself is resolved all
/// the same, because the queue is what the character class lookahead reads from.
///
/// The name is `None` for everything which isn't a command, for `\begin` and `\end` (whose
/// meaning the lexer decides, so they cannot be redefined), and for the tokens which reach the
/// queue from the body of a custom command rather than from the lexer: those carry their names
/// as [`Token::UnresolvedCommand`] instead.
#[derive(Clone, Copy, Debug)]
pub(super) struct QueuedTok<'source>(TokSpan, Option<&'source str>);

#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(QueuedTok<'static>, [usize; 7]);

impl<'source> QueuedTok<'source> {
    #[inline]
    pub(super) const fn new(tokspan: TokSpan, name: Option<&'source str>) -> Self {
        QueuedTok(tokspan, name)
    }

    #[inline]
    pub(super) fn token(&self) -> &Token {
        self.0.token()
    }

    #[inline]
    pub(super) fn tokspan(&self) -> &TokSpan {
        &self.0
    }

    #[inline]
    pub(super) fn into_tokspan(self) -> TokSpan {
        self.0
    }

    #[inline]
    pub(super) fn span(&self) -> Span {
        self.0.span()
    }

    /// The name of the command this token came from, if it came from one.
    #[inline]
    pub(super) fn name(&self) -> Option<&'source str> {
        self.1
    }

    /// Give this token a new meaning, keeping its span and the name it came from.
    #[inline]
    fn with_token(&self, token: Token) -> Self {
        QueuedTok(TokSpan::new(token, self.0.span()), self.1)
    }

    /// Unwrap a [`Token::MathOrTextMode`], as [`Token::unwrap_math`] does.
    #[inline]
    fn unwrap_math(self) -> Self {
        let (tok, span) = self.0.into_parts();
        QueuedTok(TokSpan::new(tok.unwrap_math(), span), self.1)
    }
}

impl From<Token> for QueuedTok<'_> {
    #[inline]
    fn from(token: Token) -> Self {
        QueuedTok(token.into(), None)
    }
}

impl From<TokSpan> for QueuedTok<'_> {
    #[inline]
    fn from(tokspan: TokSpan) -> Self {
        QueuedTok(tokspan, None)
    }
}

impl From<QueuedTok<'_>> for TokSpan {
    #[inline]
    fn from(queued: QueuedTok<'_>) -> Self {
        queued.0
    }
}

/// The stores which the token queue uses to resolve command names.
///
/// This is a separate struct because of lifetime issues.
pub(super) struct Stores<'state, 'arena> {
    pub parser_cfg: &'arena ParserConfig,
    pub global_state: &'state mut GlobalState,
    /// The commands which the snippet defines for itself with `\newcommand`, when the
    /// conversion does *not* run in the global group. They are dropped together with the
    /// queue, which is why they don't outlive the snippet.
    pub local_cmds: CustomCmds,
}

impl Stores<'_, '_> {
    /// Give a command name (without the leading backslash) its meaning.
    ///
    /// Returns `None` if the name isn't defined anywhere, in which case it may still be
    /// defined later on.
    fn resolve_command(&self, name: &str) -> Option<Token> {
        let local = &self.local_cmds;
        let document = &self.global_state.custom_cmds;
        let config = &self.parser_cfg.custom_cmds_from_cfg;
        // First check the stores, from most local to most global.
        if let Some(tok) = local
            .get(name, CmdSource::Local)
            .or_else(|| document.get(name, CmdSource::Document))
            .or_else(|| config.get(name, CmdSource::Config))
        {
            return Some(tok);
        }
        // Then check the built-in commands.
        resolve_builtin_cmd(self.parser_cfg, name)
    }

    /// Get the body of a command which is defined in one of the stores.
    pub(crate) fn get_body(&self, source: CmdSource, start: usize, end: usize) -> Option<&[Token]> {
        match source {
            CmdSource::Config => self.parser_cfg.custom_cmds_from_cfg.body(start, end),
            CmdSource::Document => self.global_state.custom_cmds.body(start, end),
            CmdSource::Local => self.local_cmds.body(start, end),
        }
    }

    /// The name of an unresolved command, taken from the pool which belongs to its [`CmdSource`].
    pub(crate) fn name_in_pool(&self, source: CmdSource, name: InternedStr) -> &str {
        match source {
            CmdSource::Config => self.parser_cfg.custom_cmds_from_cfg.cmd_names(),
            CmdSource::Document => self.global_state.custom_cmds.cmd_names(),
            CmdSource::Local => self.local_cmds.cmd_names(),
        }
        .get(name)
    }
}

static EOI_TOK: QueuedTok<'static> =
    QueuedTok::new(TokSpan::new(Token::Eoi, Span::zero_width(0)), None);

impl<'state, 'arena> TokenQueue<'state, 'arena> {
    pub(super) fn new(
        lexer: Lexer<'arena>,
        parser_cfg: &'arena ParserConfig,
        global_state: &'state mut GlobalState,
    ) -> Result<Self, Box<LatexError>> {
        let mut tm = TokenQueue {
            lexer,
            stores: Stores {
                parser_cfg,
                global_state,
                local_cmds: CustomCmds::default(),
            },
            queue: VecDeque::with_capacity(2),
            lexer_is_eoi: false,
            next_non_whitespace: 0,
        };
        // Ensure that we have at least one non-whitespace token in the buffer for peeking.
        let idx = tm.load_token_skip_whitespace()?;
        tm.next_non_whitespace = idx;
        Ok(tm)
    }

    /// Resolve the lexer output into a token.
    ///
    /// The lexer output is either a token or a command name. We look up the command name and if we
    /// can't find it, we return an unresolved command token with an interned name. Either way, the
    /// name is kept alongside the token; see [`QueuedTok`].
    fn resolve_lexed(&mut self, lexed: LexerOutput<'arena>) -> QueuedTok<'arena> {
        match lexed {
            LexerOutput::Token(tokspan) => tokspan.into(),
            LexerOutput::CommandName(name, span) => {
                let tok = self.stores.resolve_command(name).unwrap_or_else(|| {
                    Token::UnresolvedCommand(CmdSource::Local, self.stores.local_cmds.intern(name))
                });
                QueuedTok::new(TokSpan::new(tok, span), Some(name))
            }
        }
    }

    /// Try to give a command which was unresolved when it was read its meaning now.
    ///
    /// Returns `None` if the name is still not defined anywhere.
    fn resolve_stored(&self, source: CmdSource, name: InternedStr) -> Option<Token> {
        self.stores
            .resolve_command(self.stores.name_in_pool(source, name))
    }

    /// Load the next non-whitespace token from the lexer into the buffer, and return its index.
    fn load_token_skip_whitespace(&mut self) -> Result<usize, Box<LatexError>> {
        Ok(self
            .load_token(is_not_whitespace)?
            .unwrap_or(self.queue.len()))
    }

    /// Load the next not-skipped token from the lexer into the buffer.
    /// If the end of the input is reached, this will return early.
    fn load_token<T>(
        &mut self,
        predicate: fn(usize, &Token) -> Option<T>,
    ) -> Result<Option<T>, Box<LatexError>> {
        if self.lexer_is_eoi {
            return Ok(None);
        }
        let starting_len = self.queue.len();
        let mut non_skipped_offset = 0usize;
        loop {
            let tok = self.lexer.next_token()?;
            // Commands are resolved right away, so that no unresolved command name is ever
            // queued: the queue is what the character class lookahead reads from.
            let tok = self.resolve_lexed(tok);
            let result = predicate(starting_len + non_skipped_offset, tok.token());
            let is_eoi = matches!(tok.token(), Token::Eoi);
            self.queue.push_back(tok);
            if let Some(result) = result {
                return Ok(Some(result));
            }
            non_skipped_offset += 1;
            if is_eoi {
                self.lexer_is_eoi = true;
                return Ok(None);
            }
        }
    }

    /// Perform a linear search to find the next non-whitespace token in the buffer.
    fn find_next_non_whitespace(&self) -> Option<usize> {
        self.queue
            .iter()
            .position(|tokspan| !matches!(tokspan.token(), Token::Whitespace))
    }

    /// Ensure that `next_non_whitespace` points to the next non-whitespace token in the buffer,
    /// or to one past the end if there is none.
    fn ensure_next_non_whitespace(&mut self) -> Result<(), Box<LatexError>> {
        let pos = 'pos_calc: {
            // First, try to find the next non-whitespace token in the existing buffer.
            if !self.queue.is_empty()
                && let Some(pos) = self.find_next_non_whitespace()
            {
                break 'pos_calc pos;
            }
            // Then, try to load more tokens until we find one or reach EOI.
            self.load_token_skip_whitespace()?
        };
        self.next_non_whitespace = pos;
        Ok(())
    }

    /// Peek at the next non-whitespace token without consuming it.
    ///
    /// If the lexer has reached the end of the input, this will return an EOI token.
    /// The public interface of [`TokenQueue`] enforces the invariant that there is
    /// always at least one non-whitespace token in the buffer when this is called,
    /// unless EOI has been reached.
    ///
    /// This may return [`Token::UnresolvedCommand`] even when
    /// [`ParserConfig::ignore_unknown_commands`] is set to `false`, in which case the subsequent
    /// call to [`Self::next`] will return an error.
    #[inline]
    pub(super) fn peek(&self) -> &TokSpan {
        // `next_non_whitespace` points to the next non-whitespace token,
        // or to one past the end of the buffer if there is none.
        if let Some(tok) = self.queue.get(self.next_non_whitespace) {
            tok.tokspan()
        } else {
            debug_assert!(self.lexer_is_eoi, "peek called without ensure");
            EOI_TOK.tokspan()
        }
    }

    /// Peek at the next token without consuming it, including whitespace
    /// and [`Token::MathOrTextMode`].
    #[inline]
    pub(super) fn peek_any_token(&self) -> &TokSpan {
        self.peek_any_keeping_name().tokspan()
    }

    /// Same as [`Self::peek_any_token`], but keeps the name the command came from.
    #[inline]
    fn peek_any_keeping_name(&self) -> &QueuedTok<'arena> {
        if let Some(tok) = self.queue.front() {
            tok
        } else {
            debug_assert!(self.lexer_is_eoi, "peek called without ensure");
            &EOI_TOK
        }
    }

    /// Find or load a token which is not skipped according to `predicate`.
    ///
    /// This function starts its search after `next_non_whitespace` (i.e., it skips
    /// the first non-whitespace token). The idea is that the caller has already
    /// checked `next_non_whitespace` or is not interested in it.
    fn find_or_load_after_next<T>(
        &mut self,
        predicate: fn(usize, &Token) -> Option<T>,
    ) -> Result<Option<T>, Box<LatexError>> {
        // We use a block here which returns an index to avoid borrow checker issues.
        let result = {
            // Ensure that the compiler can tell that `self.queue.range(start..)`
            // cannot panic due to being out of bounds.
            let start = self.next_non_whitespace;
            if start < self.queue.len() {
                let mut range = self.queue.range(start..);
                range.next(); // Skip `next_non_whitespace`.
                range
                    .enumerate()
                    .find_map(|(idx, ts)| predicate(start + 1 + idx, ts.token()))
            } else {
                debug_assert!(
                    self.lexer_is_eoi,
                    "find_or_load_after_next called without ensure"
                );
                return Ok(None);
            }
        };

        if let Some(result) = result {
            // If we found a token in the existing buffer, return it.
            Ok(Some(result))
        } else {
            // Otherwise, load more tokens until we find one or reach EOI.
            self.load_token(predicate)
        }
    }

    /// Peek at the second non-whitespace token without consuming it.
    pub(super) fn peek_second(&mut self) -> Result<&TokSpan, Box<LatexError>> {
        if let Some(tok) = self
            .find_or_load_after_next(is_not_whitespace)?
            .and_then(|idx| self.queue.get(idx))
        {
            Ok(tok.tokspan())
        } else {
            debug_assert!(self.lexer_is_eoi, "peek_second called without ensure");
            Ok(EOI_TOK.tokspan())
        }
    }

    /// Peek at the first token which has a character class.
    ///
    /// This excludes, for example, `Space` tokens.
    pub(super) fn peek_class_token(&mut self) -> Result<(usize, Class), Box<LatexError>> {
        // First check the common case where the next token is already a token with class.
        if let Some(class) = self.peek().token().class() {
            Ok((self.next_non_whitespace, class))
        } else if let Some(class) = self.find_or_load_after_next(has_class)? {
            Ok(class)
        } else {
            debug_assert!(self.lexer_is_eoi, "peek_class_token called without ensure");
            Ok((self.queue.len(), Class::End))
        }
    }

    /// Reject a command which is still not defined, unless the configuration says to render it
    /// instead.
    ///
    /// Resolution deliberately does not decide what to do with a name it cannot find, because
    /// the name of one can be something we want to use rather than reject. Doing it here
    /// means that unknown commands are reported as such no matter where they show up,
    /// instead of the consumer having to report whatever it expected in that position.
    fn reject_unknown_command(&self, tokspan: &TokSpan) -> Result<(), Box<LatexError>> {
        if let Token::UnresolvedCommand(source, name) = *tokspan.token()
            && !self.stores.parser_cfg.ignore_unknown_commands
        {
            // The name lives in one of the string pools, so we have to copy it out.
            return Err(Box::new(LatexError(
                tokspan.span().into(),
                LatexErrKind::UnknownCommand(self.stores.name_in_pool(source, name).into()),
            )));
        }
        Ok(())
    }

    /// Get the next math-mode token.
    ///
    /// This method skips any whitespace tokens and unwraps [`Token::MathOrTextMode`].
    ///
    /// This method also ensures that there is always a peekable token after this one.
    pub(super) fn next(&mut self) -> Result<TokSpan, Box<LatexError>> {
        Ok(self.next_keeping_name()?.into_tokspan())
    }

    /// Same as [`Self::next`], but keeps the name the command came from.
    pub(super) fn next_keeping_name(&mut self) -> Result<QueuedTok<'arena>, Box<LatexError>> {
        let ret = self.next_allowing_unknown_command()?;
        self.reject_unknown_command(ret.tokspan())?;
        Ok(ret)
    }

    /// Same as [`Self::next`], but unknown commands are returned as tokens instead of
    /// being rejected.
    ///
    /// This is for the places which want to get hold of the name of a command which is
    /// not defined (yet).
    pub(super) fn next_allowing_unknown_command(
        &mut self,
    ) -> Result<QueuedTok<'arena>, Box<LatexError>> {
        // Pop elements until we reach `next_non_whitespace`.
        for _ in 0..self.next_non_whitespace {
            let _ = self.queue.pop_front();
        }

        // Now pop the next token.
        if let Some(ret) = self.queue.pop_front() {
            self.ensure_next_non_whitespace()?;
            let ret = ret.unwrap_math();
            debug_assert!(!matches!(
                ret.token(),
                Token::Whitespace | Token::MathOrTextMode(_, _)
            ));
            Ok(ret)
        } else {
            // We must have reached EOI previously.
            debug_assert!(self.lexer_is_eoi, "next called without ensure");
            Ok(EOI_TOK)
        }
    }

    /// Get the next token without skipping or unwrapping anything.
    ///
    /// This method may return whitespace tokens and [`Token::MathOrTextMode`].
    pub(super) fn next_any_token(&mut self) -> Result<TokSpan, Box<LatexError>> {
        Ok(self.next_any_token_keeping_name()?.into_tokspan())
    }

    /// Same as [`Self::next_any_token`], but keeps the name the command came from.
    fn next_any_token_keeping_name(&mut self) -> Result<QueuedTok<'arena>, Box<LatexError>> {
        let ret = self.next_any_token_allowing_unknown_command()?;
        self.reject_unknown_command(ret.tokspan())?;
        Ok(ret)
    }

    /// Same as [`Self::next_any_token`], but commands which are not defined (yet) are returned
    /// as tokens instead of being rejected.
    pub(super) fn next_any_token_allowing_unknown_command(
        &mut self,
    ) -> Result<QueuedTok<'arena>, Box<LatexError>> {
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

    /// Queue a stream of tokens in the front of the buffer.
    ///
    /// We use a ring buffer, so this is efficient as long as the number of tokens is not too large.
    pub(super) fn queue_in_front(&mut self, tokens: &[impl Into<QueuedTok<'arena>> + Copy]) {
        self.queue.reserve(tokens.len());
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            self.queue.push_front((*tok).into());
        }
        self.update_next_non_whitespace();
    }

    /// Update the `next_non_whitespace` position after tokens have been queued in front.
    fn update_next_non_whitespace(&mut self) {
        if let Some(pos) = self.find_next_non_whitespace() {
            self.next_non_whitespace = pos;
        } else {
            // There is only one scenario in which we wouldn't find a non-whitespace token:
            // We reached EOI previously and all queued tokens are whitespace.
            debug_assert!(self.lexer_is_eoi, "queued in front without ensure");
            self.next_non_whitespace = self.queue.len();
        }
    }

    /// Queue the body of a custom command in the front of the buffer, substituting the
    /// arguments of the command for the [`Token::CustomCmdArg`] tokens in it.
    ///
    /// The substitution happens here, when the body is queued, rather than when the parser
    /// gets to the argument tokens. Doing it eagerly means that no `CustomCmdArg` is ever
    /// queued, so the arguments are no longer needed once this returns. That is what makes it
    /// possible for the body of a command to contain another command which takes arguments of
    /// its own.
    ///
    /// The `span` of the command being expanded is used for the tokens which are inserted
    /// for something that isn't there: the braces of an empty argument, and the `\relax` of
    /// an empty body.
    ///
    /// Always queues at least one token, so that the caller can parse the expansion by
    /// simply taking the next token.
    pub(super) fn queue_body_substituting(
        &mut self,
        tokens: &[impl Into<QueuedTok<'arena>> + Copy],
        args: &CmdArgs<'arena>,
        span: Span,
    ) {
        if tokens.is_empty() {
            // A body which is empty produces nothing at all, but we still have to queue
            // something: without it, the caller would take the token which comes after the
            // command and treat that as the expansion.
            self.queue
                .push_front(TokSpan::new(Token::Relax, span).into());
            self.update_next_non_whitespace();
            return;
        }
        self.queue.reserve(tokens.len());
        // Queue the token stream in the front in reverse order.
        for tok in tokens.iter().rev() {
            let queued: QueuedTok<'arena> = (*tok).into();
            match *queued.token() {
                Token::CustomCmdArg(arg_num) => self.push_arg_front(args.get(arg_num), span),
                // A command which the body only refers to by name gets its meaning here, so a
                // command which didn't exist when the body was recorded may exist by now, and
                // one which has been redefined since means something else now.
                Token::UnresolvedCommand(source, name) => {
                    let resolved = self.resolve_stored(source, name);
                    // A body carries no spans of its own, so if the command is still not
                    // defined, the error has to point at the command we are expanding.
                    let tok = resolved.unwrap_or(*queued.token());
                    let tok_span = if resolved.is_some() {
                        queued.span()
                    } else {
                        span
                    };
                    self.queue
                        .push_front(QueuedTok::new(TokSpan::new(tok, tok_span), queued.name()));
                }
                _ => self.queue.push_front(queued),
            }
        }
        self.update_next_non_whitespace();
    }

    /// Queue the tokens of one argument of a custom command in the front of the buffer.
    ///
    /// An argument with no tokens is queued as an empty group, because an empty argument is
    /// equivalent to `{}`. Substituting nothing at all would make it disappear from
    /// constructs which need an argument, which would then take whatever comes next instead.
    /// The `span` is only used for the braces of that empty group.
    fn push_arg_front(&mut self, arg: &[QueuedTok<'arena>], span: Span) {
        if arg.is_empty() {
            self.queue
                .push_front(TokSpan::new(Token::GroupEnd, span).into());
            self.queue
                .push_front(TokSpan::new(Token::GroupBegin, span).into());
        } else {
            for queued in arg.iter().rev() {
                self.queue.push_front(*queued);
            }
        }
    }

    /// Queue one argument of a custom command in the front of the buffer, as
    /// [`Self::push_arg_front`] does. Always queues at least one token.
    pub(super) fn queue_arg_in_front(&mut self, arg: &[QueuedTok<'arena>], span: Span) {
        self.queue.reserve(arg.len());
        self.push_arg_front(arg, span);
        self.update_next_non_whitespace();
    }

    /// Resolve buffered tokens for commands which have been defined in the meantime.
    ///
    /// Commands are normally resolved when they are read, but the buffer may already contain
    /// the token of a command which is only defined once we get to it: the token right after
    /// a `\newcommand` has already been read by then.
    pub(super) fn resolve_buffered_unknown_commands(&mut self) {
        for queued in &mut self.queue {
            if let Token::UnresolvedCommand(source, name) = *queued.token()
                && let Some(tok) = self
                    .stores
                    .resolve_command(self.stores.name_in_pool(source, name))
            {
                *queued = queued.with_token(tok);
            }
        }
    }

    /// Register a custom command, and make the tokens which are already buffered aware of it.
    ///
    /// The definition either goes into the store which outlives this snippet, or into the one
    /// which doesn't: `global` says that this particular definition was made global with
    /// `\global`, and [`ParserConfig::global_group`] says that all of them are. Returns `false`
    /// if `replace` is not set and the name is already taken.
    pub(super) fn define(
        &mut self,
        name: &str,
        num_args: u8,
        body: &[Token],
        first_class: Option<Class>,
        replace: bool,
        is_global: bool,
    ) -> bool {
        let is_global = is_global || self.stores.parser_cfg.global_group;
        let source = if is_global {
            CmdSource::Document
        } else {
            CmdSource::Local
        };
        let stores = &mut self.stores;
        if is_global {
            // The body has to outlive the snippet, so anything it refers to in the local store
            // is copied into the store of the global state along with it.
            let local = &stores.local_cmds;
            let document = &mut stores.global_state.custom_cmds;
            if replace {
                document.insert_or_replace_and_copy_local(local, name, num_args, body, first_class);
            } else if !document.insert_and_copy_local(local, name, num_args, body, first_class) {
                return false;
            }
            // A global definition takes effect right away, as it does in TeX, so a local
            // definition of the same name must not go on shadowing it.
            stores.local_cmds.remove(name);
        } else if replace {
            stores
                .local_cmds
                .insert_or_replace(name, num_args, body, first_class);
        } else if !stores.local_cmds.insert(name, num_args, body, first_class) {
            return false;
        }
        let store = match source {
            CmdSource::Document => &self.stores.global_state.custom_cmds,
            _ => &self.stores.local_cmds,
        };
        if replace {
            // The token after the definition has already been loaded, so it may still refer
            // to the definition we have just replaced.
            if let Some(tok) = store.get(name, source) {
                self.resolve_buffered_redefined_command(name, tok);
            }
        } else {
            // The token after the definition has already been loaded, so it may still say
            // "unknown command" for the command we have just defined.
            self.resolve_buffered_unknown_commands();
        }
        true
    }

    /// Update buffered tokens for a command which has just been redefined.
    ///
    /// The reason is the same as for [`Self::resolve_buffered_unknown_commands`]: the token
    /// after a `\renewcommand` has already been loaded, so a use of the command right after
    /// its redefinition would otherwise still refer to the old definition. We recognize the
    /// tokens by the name they came from, because the old definition can be any token at all.
    pub(super) fn resolve_buffered_redefined_command(&mut self, name: &str, tok: Token) {
        for queued in &mut self.queue {
            if queued.name() == Some(name) {
                *queued = queued.with_token(tok);
            }
        }
    }

    /// Record the body of a command defined with `\newcommand`.
    ///
    /// The next token must be the opening `{`, which this consumes; the closing `}` is left
    /// for the caller. Both of those are dictated by the one-token lookahead: macro
    /// parameters have to be allowed before the token after the `{` is loaded from the lexer,
    /// and disallowed again before the token after the `}` is.
    ///
    /// Every command keeps the name it came from, so that the body can be recorded by name
    /// rather than by what those names mean right now. A command which isn't defined (yet) is
    /// therefore recorded rather than rejected: it may well be defined by the time the command
    /// being defined here is used.
    pub(super) fn record_macro_body(
        &mut self,
        tokens: &mut Vec<QueuedTok<'arena>>,
    ) -> Result<(), Box<LatexError>> {
        core::debug_assert_matches!(self.peek().token(), Token::GroupBegin);
        self.next()?; // Discard the opening `{`.
        let mut nesting_level = 0usize;
        loop {
            let tokloc = *self.peek_any_keeping_name();
            match tokloc.token() {
                Token::GroupBegin => {
                    nesting_level += 1;
                }
                Token::GroupEnd => {
                    // If the nesting level reaches one below where we started, we stop
                    // reading, leaving the `}` for the caller.
                    let Some(new_level) = nesting_level.checked_sub(1) else {
                        return Ok(());
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
            self.next_any_token_allowing_unknown_command()?;
            tokens.push(tokloc);
        }
    }

    /// Read a group of tokens, ending with (an unopened) `}`.
    ///
    /// The initial `{` must have already been consumed. The closing `}` is not included
    /// in the output token vector.
    pub(super) fn record_group<T: From<QueuedTok<'arena>>>(
        &mut self,
        tokens: &mut Vec<T>,
        preserve_all: bool,
    ) -> Result<usize, Box<LatexError>> {
        let mut nesting_level = 0usize;
        let end = loop {
            let tokloc = if preserve_all {
                self.next_any_token_keeping_name()
            } else {
                self.next_keeping_name()
            };
            let tokloc = tokloc?;
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
            tokens.push(tokloc.into());
        };
        Ok(end)
    }

    /// Read one macro argument, which is either a single token or a group of tokens.
    ///
    /// Any immediately following whitespace is always skipped. If the argument is a group, then
    /// the parameter `preserve_all` determines whether the whitespace tokens within the group
    /// are included in the output vector or not.
    pub fn read_argument(&mut self, preserve_all: bool) -> Result<MacroArgument, Box<LatexError>> {
        let first = if preserve_all {
            // For `preserve_all`, we still want to skip leading whitespace, but we don't want to
            // perform the unwrapping that `next()` does. So we use this hack here of copying the
            // peek token and then discarding it with `next()`.
            let tok = *self.peek();
            self.next()?;
            tok
        } else {
            self.next()?
        };
        if matches!(first.token(), Token::GroupBegin) {
            let mut tokens = Vec::new();
            // Read until the matching `}`.
            let end_loc = self.record_group(&mut tokens, preserve_all)?;
            Ok(MacroArgument::Group(tokens, first.span().start()..end_loc))
        } else {
            Ok(MacroArgument::Token(first))
        }
    }

    /// Get a token from the buffer by its index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub(super) fn get_token_by_index(&self, idx: usize) -> Option<&TokSpan> {
        self.queue.get(idx).map(QueuedTok::tokspan)
    }
}

fn is_not_whitespace(idx: usize, tok: &Token) -> Option<usize> {
    (!matches!(tok, Token::Whitespace)).then_some(idx)
}

fn has_class(idx: usize, tok: &Token) -> Option<(usize, Class)> {
    tok.class().map(|class| (idx, class))
}

/// The arguments of the custom command which is currently being expanded.
///
/// The tokens of all arguments are kept in one flat vector; `offsets[i]` is where the
/// argument with index `i` ends. The buffer is reused for the whole parse, and it is only
/// alive for as long as it takes to queue the body of the command: because the arguments are
/// substituted into the body right away, nothing refers to them afterwards.
#[derive(Debug, Default)]
pub(super) struct CmdArgs<'source> {
    tokens: Vec<QueuedTok<'source>>,
    offsets: [usize; 9],
}

impl<'source> CmdArgs<'source> {
    /// The tokens which make up the given argument.
    pub(super) fn get(&self, arg_num: u8) -> &[QueuedTok<'source>] {
        let start = self
            .offsets
            .get(arg_num.wrapping_sub(1) as usize)
            .copied()
            .unwrap_or(0);
        let end = self
            .offsets
            .get(arg_num as usize)
            .copied()
            .unwrap_or(self.tokens.len());
        self.tokens.get(start..end).unwrap_or(&[])
    }

    /// Forget the arguments of the command which was expanded before this one.
    pub(super) fn clear(&mut self) {
        self.tokens.clear();
        self.offsets = [0; 9];
    }

    /// Append a token to the argument which is currently being read.
    pub(super) fn push(&mut self, queued: QueuedTok<'source>) {
        self.tokens.push(queued);
    }

    /// Mark the end of the argument with the given index.
    pub(super) fn finish_arg(&mut self, arg_num: u8) {
        if let Some(offset) = self.offsets.get_mut(arg_num as usize) {
            *offset = self.tokens.len();
        }
    }

    /// The buffer to read the tokens of an argument into.
    pub(super) fn buffer(&mut self) -> &mut Vec<QueuedTok<'source>> {
        &mut self.tokens
    }
}

/// A macro argument, which is either a single token or a group of tokens.
pub enum MacroArgument {
    Token(TokSpan),
    /// The `Range` is the range of the entire group, including the opening and closing braces.
    Group(Vec<TokSpan>, Range<usize>),
}

impl MacroArgument {
    /// Try to interpret this macro argument as a single token.
    pub fn into_one_or_none(self) -> Result<OneOrNone, Box<LatexError>> {
        match self {
            MacroArgument::Token(tok) => Ok(OneOrNone::One(tok)),
            MacroArgument::Group(tokens, span) => {
                if tokens.is_empty() {
                    Ok(OneOrNone::None(span))
                } else if let Ok([tokspan]) = <[TokSpan; 1]>::try_from(tokens) {
                    Ok(OneOrNone::One(tokspan))
                } else {
                    Err(Box::new(LatexError(
                        span,
                        LatexErrKind::ExpectedAtMostOneToken,
                    )))
                }
            }
        }
    }

    /// Try to interpret this macro argument as a single token.
    pub fn into_one(self) -> Result<TokSpan, Box<LatexError>> {
        match self {
            MacroArgument::Token(tok) => Ok(tok),
            MacroArgument::Group(tokens, span) => {
                if let Ok([tokspan]) = <[TokSpan; 1]>::try_from(tokens) {
                    Ok(tokspan)
                } else {
                    Err(Box::new(LatexError(
                        span,
                        LatexErrKind::ExpectedExactlyOneToken,
                    )))
                }
            }
        }
    }
}

pub enum OneOrNone {
    One(TokSpan),
    None(Range<usize>),
}

impl From<OneOrNone> for Option<TokSpan> {
    fn from(value: OneOrNone) -> Self {
        match value {
            OneOrNone::One(tok) => Some(tok),
            OneOrNone::None(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn test_record_group() {
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

        let parser_cfg = crate::ParserConfig::default();
        let mut state = crate::GlobalState::default();
        for (name, problem) in problems.into_iter() {
            let lexer = Lexer::new(problem);
            let mut manager = TokenQueue::new(lexer, &parser_cfg, &mut state)
                .expect("Failed to create TokenManager");
            // Load up some tokens to ensure the code can deal with that.
            manager.load_token_skip_whitespace().unwrap();
            manager.load_token_skip_whitespace().unwrap();
            // Check that the first token is `GroupBegin`.
            std::assert_matches!(manager.next().unwrap().token(), Token::GroupBegin);
            let mut tokens: Vec<TokSpan> = Vec::new();
            let tokens = match manager.record_group(&mut tokens, true) {
                Ok(_) => {
                    let mut token_str = String::new();
                    for tokloc in tokens {
                        let (tok, span) = tokloc.into_parts();
                        writeln!(token_str, "{}..{}: {:?}", span.start(), span.end(), tok).unwrap();
                    }
                    token_str
                }
                Err(error) => {
                    let report = error.to_report("<input>", false);
                    let mut buf = Vec::new();
                    report
                        .write(("<input>", ariadne::Source::from(problem)), &mut buf)
                        .expect("failed to write report");
                    String::from_utf8(buf).expect("report should be valid UTF-8")
                }
            };
            assert_snapshot!(name, &tokens, problem);
        }
    }

    #[test]
    fn test_get_whitespace_tokens() {
        let input = r"\text{  x +   y }";
        // let input = r"\text  xy";
        let parser_cfg = crate::ParserConfig::default();
        let mut state = crate::GlobalState::default();
        let lexer = Lexer::new(input);
        let mut manager =
            TokenQueue::new(lexer, &parser_cfg, &mut state).expect("Failed to create TokenManager");

        let mut token_str = String::new();

        loop {
            let (tok, span) = manager.next_any_token().unwrap().into_parts();
            if matches!(tok, Token::Eoi) {
                break;
            }
            writeln!(token_str, "{}..{}: {:?}", span.start(), span.end(), tok).unwrap();
        }

        assert_snapshot!("next_with_whitespace", &token_str, input);
    }

    #[test]
    fn test_find_or_load_after_next() {
        let input = r"x y z";
        // let input = r"\text  xy";
        let parser_cfg = crate::ParserConfig::default();
        let mut state = crate::GlobalState::default();
        let lexer = Lexer::new(input);
        let mut queue =
            TokenQueue::new(lexer, &parser_cfg, &mut state).expect("Failed to create TokenManager");
        queue.next().unwrap(); // Consume 'x'
        assert_eq!(queue.next_non_whitespace, 1);
        assert_eq!(queue.queue.len(), 2);
        std::assert_matches!(queue.queue[0].token(), Token::Whitespace);
        assert!(
            matches!(queue.peek().token(), Token::Letter(c, _) if c.try_as_char() == Some('y'))
        );

        // Test the branch that needs to load more tokens.
        let tok_idx = queue.find_or_load_after_next(is_not_whitespace).unwrap();
        std::assert_matches!(tok_idx, Some(3));
        assert_eq!(queue.queue.len(), 4);
        std::assert_matches!(queue.queue[0].token(), Token::Whitespace);
        std::assert_matches!(queue.queue[2].token(), Token::Whitespace);
        assert!(
            matches!(queue.queue[3].token(), Token::Letter(c, _) if c.try_as_char() == Some('z'))
        );

        // Test the branch that finds the token in the existing buffer.
        let tok_idx = queue.find_or_load_after_next(is_not_whitespace).unwrap();
        std::assert_matches!(tok_idx, Some(3));
        assert_eq!(queue.queue.len(), 4);
        std::assert_matches!(queue.queue[0].token(), Token::Whitespace);
        std::assert_matches!(queue.queue[2].token(), Token::Whitespace);
        assert!(
            matches!(queue.queue[3].token(), Token::Letter(c, _)if c.try_as_char() == Some('z'))
        );
    }

    #[test]
    fn text_read_argument() {
        let problems = [
            ("hyphen", r"-xy", true),
            ("hyphen_math_mode", r"-xy", false),
            ("consecutive_whitespace", r"{x   y} z", true),
            ("consecutive_whitespace_skip", r"{x   y} z", false),
        ];

        let parser_cfg = crate::ParserConfig::default();
        for (name, problem, preserve_all) in problems.into_iter() {
            let mut state = crate::GlobalState::default();
            let lexer = Lexer::new(problem);
            let mut manager = TokenQueue::new(lexer, &parser_cfg, &mut state)
                .expect("Failed to create TokenManager");
            let tokens = match manager.read_argument(preserve_all) {
                Ok(MacroArgument::Group(tokens, _)) => {
                    let mut token_str = String::new();
                    for tokloc in tokens {
                        let (tok, span) = tokloc.into_parts();
                        writeln!(token_str, "{}..{}: {:?}", span.start(), span.end(), tok).unwrap();
                    }
                    token_str
                }
                Ok(MacroArgument::Token(tok)) => {
                    let (tok, span) = tok.into_parts();
                    format!("{}..{}: {:?}\n", span.start(), span.end(), tok)
                }
                Err(error) => {
                    let report = error.to_report("<input>", false);
                    let mut buf = Vec::new();
                    report
                        .write(("<input>", ariadne::Source::from(problem)), &mut buf)
                        .expect("failed to write report");
                    String::from_utf8(buf).expect("report should be valid UTF-8")
                }
            };
            assert_snapshot!(name, &tokens, problem);
        }
    }
}
