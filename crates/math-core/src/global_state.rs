use rustc_hash::FxHashMap;

#[derive(Debug, Default)]
pub(crate) struct GlobalState {
    /// This is used for numbering equations in the document.
    pub(crate) equation_count: usize,
    /// This is used for resolving references to equations in the document. The keys are the labels
    /// defined in the document, and the values are the corresponding equation numbers (as strings).
    pub(crate) label_map: FxHashMap<Box<str>, Box<str>>,
}
