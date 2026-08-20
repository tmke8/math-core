use alloc::vec::Vec;

use kstring::KString;
use rustc_hash::FxBuildHasher;

use crate::FxHashMap;
use crate::character_class::Class;
use crate::token::Token;

/// Where the token stream of a custom command is stored.
///
/// The stores are kept separately, because they have different lifetimes: one is part of the
/// configuration and therefore immutable, one is filled while a document is being parsed, and
/// one only lives as long as the snippet which defines its commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdSource {
    /// The commands defined in [`MathCoreConfig::macros`](crate::MathCoreConfig::macros).
    Config,
    /// The commands which the document defines with `\newcommand` in the global group; they
    /// stay defined for the snippets which follow.
    Document,
    /// The commands which the document defines with `\newcommand` in a local group; they are
    /// forgotten at the end of the snippet which defines them.
    Local,
}

/// The definition of a custom command.
///
/// The token stream which makes up the body is stored separately, in [`CustomCmds::tokens`];
/// `start` and `end` delimit the slice belonging to this command.
#[derive(Debug)]
struct CmdDef {
    num_args: u8,
    /// The character class of the body, which we have to remember because
    /// [`Token::class`] cannot look into the store.
    ///
    /// The class is the one the body had when it was recorded, and it does not follow a later
    /// redefinition of the command the body begins with, even though the *meaning* of that
    /// command does. A body which begins with a command that isn't defined yet counts as an
    /// ordinary atom, because the class has to be known here, before we can know what that
    /// command will turn out to be.
    class: Option<Class>,
    start: usize,
    end: usize,
}

/// A collection of custom commands.
///
/// The bodies of all commands are stored in one flat vector, and the map only holds indices
/// into it. Indices rather than references, because a store which referred to itself would be
/// self-referential; this way, the body of one command can mention another command.
#[derive(Debug, Default)]
pub(crate) struct CustomCmds {
    tokens: Vec<Token>,
    map: FxHashMap<KString, CmdDef>,
}

impl CustomCmds {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        CustomCmds {
            tokens: Vec::new(),
            map: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
        }
    }

    /// Look up a command by name, returning the token which refers to its body.
    pub(crate) fn get(&self, name: &str, source: CmdSource) -> Option<Token> {
        let def = self.map.get(name)?;
        Some(Token::CustomCmdRef(
            source,
            def.num_args,
            def.class,
            def.start,
            def.end,
        ))
    }

    /// Get the body of a command, given the range stored in its [`Token::CustomCmdRef`].
    pub(crate) fn body(&self, start: usize, end: usize) -> Option<&[Token]> {
        self.tokens.get(start..end)
    }

    /// Define a new command, returning `false` if a command of that name already exists.
    pub(crate) fn insert(
        &mut self,
        name: &str,
        num_args: u8,
        body: &[Token],
        first_class: Option<Class>,
    ) -> bool {
        if self.map.contains_key(name) {
            return false;
        }
        self.insert_or_replace(name, num_args, body, first_class);
        true
    }

    /// Define a command, overwriting an existing definition of the same name.
    ///
    /// The body of the old definition stays in the store, because other definitions may
    /// contain references into it.
    pub(crate) fn insert_or_replace(
        &mut self,
        name: &str,
        num_args: u8,
        body: &[Token],
        first_class: Option<Class>,
    ) {
        let start = self.tokens.len();
        self.tokens.extend_from_slice(body);
        let end = self.tokens.len();
        self.register(name, num_args, first_class, start, end);
    }

    /// Like [`Self::insert`], but for a body which may refer to `local`; see [`Self::absorb`].
    pub(crate) fn insert_and_copy_local(
        &mut self,
        local: &CustomCmds,
        name: &str,
        num_args: u8,
        body: &[Token],
        first_class: Option<Class>,
    ) -> bool {
        if self.map.contains_key(name) {
            return false;
        }
        self.insert_or_replace_and_copy_local(local, name, num_args, body, first_class);
        true
    }

    /// Like [`Self::insert_or_replace`], but for a body which may refer to `local`; see
    /// [`Self::absorb`].
    pub(crate) fn insert_or_replace_and_copy_local(
        &mut self,
        local: &CustomCmds,
        name: &str,
        num_args: u8,
        body: &[Token],
        first_class: Option<Class>,
    ) {
        let (start, end) = self.absorb(local, body);
        self.register(name, num_args, first_class, start, end);
    }

    /// Register the given slice under the given name.
    fn register(
        &mut self,
        name: &str,
        num_args: u8,
        first_class: Option<Class>,
        start: usize,
        end: usize,
    ) {
        self.map.insert(
            KString::from_ref(name),
            CmdDef {
                num_args,
                class: first_class,
                start,
                end,
            },
        );
    }

    /// Copy a body into this store, translating every `local` reference as it goes.
    ///
    /// This is what `\global` needs: a definition which is about to outlive the snippet must not
    /// hold anything which points into the store that dies with the snippet. Names are re-interned
    /// in this store's pool, and the body of a command the body refers to is copied here as well,
    /// which in turn may bring further references along.
    ///
    /// Those further references are found by walking the very vector we are appending to, so the
    /// copy is iterative no matter how long a chain of `\let`s it has to follow. It terminates
    /// because the ranges in a store always point backwards — bodies are only ever appended — so
    /// there can be no cycle.
    fn absorb(&mut self, local: &CustomCmds, body: &[Token]) -> (usize, usize) {
        let start = self.tokens.len();
        self.tokens.extend_from_slice(body);
        let end = self.tokens.len();
        let mut i = start;
        // We can't use `.iter_mut()` here because we append to the vector inside the loop.
        // We also need to check the length each iteration.
        while i < self.tokens.len() {
            // A reference to a body in the local store: we copy over the body. The tokens
            // we append here to `self.tokens` are visited later by this same loop, so a
            // reference inside the copied body gets translated too.
            if let Token::CustomCmdRef(CmdSource::Local, num_args, class, body_start, body_end) =
                self.tokens[i]
            {
                let new_start = self.tokens.len();
                if let Some(body) = local.body(body_start, body_end) {
                    self.tokens.extend_from_slice(body);
                }
                let new_end = self.tokens.len();
                self.tokens[i] =
                    Token::CustomCmdRef(CmdSource::Document, num_args, class, new_start, new_end);
            }
            i += 1;
        }
        (start, end)
    }

    /// Forget the definition of a command, if there is one.
    ///
    /// Only the name is forgotten; the body stays in the store, because other definitions may
    /// contain references into it, just as with [`Self::insert_or_replace`].
    pub(crate) fn remove(&mut self, name: &str) {
        self.map.remove(name);
    }

    pub(crate) fn clear(&mut self) {
        self.tokens.clear();
        self.map.clear();
    }
}

/// Check whether a name given in the configuration can be used as a macro name.
///
/// Names coming from `\newcommand` don't need this check, because the lexer has already
/// decided what makes up a command name by the time we see one.
pub(crate) fn is_valid_macro_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        // If the name contains only one character, any character is valid.
        (Some(_), None) => true,
        // If the name contains more than one character, all characters must be ASCII alphabetic.
        _ => s.bytes().all(|b| b.is_ascii_alphabetic()),
    }
}
