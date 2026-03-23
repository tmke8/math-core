use mathml_renderer::attribute::TextTransform;

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
    /// `mathinner`
    Inner,
    /// A class indicating the end of the current formula.
    End,
}

/// <mi> mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MathVariant {
    /// This is enforced by setting `mathvariant="normal"`.
    Normal,
    /// This is enforced by transforming the characters themselves.
    Transform(TextTransform),
}

#[cfg(test)]
mod tests {
    use super::{MathVariant, TextTransform};

    #[test]
    fn size_test() {
        assert_eq!(
            std::mem::size_of::<MathVariant>(),
            std::mem::size_of::<TextTransform>()
        );
        assert_eq!(
            std::mem::size_of::<Option<MathVariant>>(),
            std::mem::size_of::<TextTransform>()
        );
    }
}
