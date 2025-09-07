#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `mathord`
    #[default]
    Default,
    /// `mathopen`
    Open,
    /// `mathclose`
    Close,
    /// `mathopen` or `mathclose`
    /// This is a temporary variant that we use because we don't always know yet
    /// if we are parsing an opening or closing symbol.
    OpenOrClose,
    /// `mathrel`
    Relation,
    /// `mathpunct`
    Punctuation,
    /// `mathbin`
    BinaryOp,
    /// `mathop`
    Operator,
}
