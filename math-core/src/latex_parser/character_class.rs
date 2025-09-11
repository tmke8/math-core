#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `mathord`
    #[default]
    Default,
    /// `mathopen`
    Open,
    /// `mathclose`
    Close,
    /// `mathrel`
    Relation,
    /// `mathpunct`
    Punctuation,
    /// `mathbin`
    BinaryOp,
    /// `mathop`
    Operator,
}
