#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `mathord`
    #[default]
    Default = 0,
    /// `mathop`
    Operator,
    /// `mathbin`
    BinaryOp,
    /// `mathrel`
    Relation,
    /// `mathopen`
    Open,
    /// `mathclose`
    Close,
    /// `mathpunct`
    Punctuation,
}
