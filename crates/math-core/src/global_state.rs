use kstring::KString;

use crate::FxHashMap;
use crate::custom_cmds::CustomCmds;

#[derive(Debug, Default)]
pub(crate) struct GlobalState {
    /// This is used for numbering equations in the document.
    pub(crate) equation_count: usize,
    /// This is used for resolving references to equations in the document. The keys are the labels
    /// defined in the document, and the values are the corresponding equation numbers (as strings).
    pub(crate) label_map: FxHashMap<KString, KString>,
    /// The commands which the document defines for itself, with `\newcommand`, when the
    /// conversion runs in the global group. They stay defined for the snippets which follow.
    pub(crate) custom_cmds: CustomCmds,
}
