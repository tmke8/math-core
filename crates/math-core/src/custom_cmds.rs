use alloc::boxed::Box;
use alloc::vec::Vec;

use rustc_hash::FxBuildHasher;

use crate::FxHashMap;
use crate::character_class::Class;
use crate::token::Token;

/// Where the token stream of a custom command is stored.
///
/// The two stores are kept separately, because one of them is part of the configuration and
/// therefore immutable, whereas the other one is filled while a document is being parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdSource {
    /// The commands defined in [`MathCoreConfig::macros`](crate::MathCoreConfig::macros).
    Config,
    /// The commands defined in the document itself, with `\newcommand`.
    Document,
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
    map: FxHashMap<Box<str>, CmdDef>,
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
        self.map.insert(
            name.into(),
            CmdDef {
                num_args,
                class: first_class,
                start,
                end,
            },
        );
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
