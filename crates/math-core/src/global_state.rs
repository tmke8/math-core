use alloc::boxed::Box;

use crate::FxHashMap;
use crate::custom_cmds::CustomCmds;

#[derive(Debug, Default)]
pub(crate) struct GlobalState {
    /// This is used for numbering equations in the document.
    pub(crate) equation_count: usize,
    /// This is used for resolving references to equations in the document. The keys are the labels
    /// defined in the document, and the values are the corresponding equation numbers (as strings).
    pub(crate) label_map: FxHashMap<Box<str>, Box<str>>,
    /// The commands which the document defines for itself, with `\newcommand`.
    ///
    /// While a snippet is being parsed, this is moved into the lexer, which is what resolves
    /// command names; the parser moves it back when it is done.
    pub(crate) custom_cmds: CustomCmds,
}
