use mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::{MathSpacing, OpAttrs, RowAttr, Style, TextTransform},
    symbol::{self, MathMLOperator, OrdCategory, OrdLike, Rel, RelCategory},
};

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

#[derive(Debug, Clone, Copy)]
pub enum ParenType {
    Left = 1,
    Right,
    Middle,
}

/// <mi> mathvariant attribute
#[derive(Debug, Clone, Copy)]
pub enum MathVariant {
    /// This is enforced by setting `mathvariant="normal"`.
    Normal,
    /// This is enforced by transforming the characters themselves.
    Transform(TextTransform),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stretchy {
    /// The operator is always stretchy (e.g. `(`, `)`).
    Always = 1,
    /// The operator is only stretchy as a pre- or postfix operator (e.g. `|`).
    PrePostfix,
    /// The operator is never stretchy (e.g. `/`).
    Never,
    /// The operator is always stretchy but isn't symmetric (e.g. `↑`).
    AlwaysAsymmetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterSpacing {
    /// Never has any spacing, even when used as an infix operator (e.g. `(`, `)`).
    Zero,
    /// Has relation spacing when used as an infix operator, but not when used as a prefix or
    /// postfix operator (e.g. `|`).
    InfixRelation,
    /// Always has relation spacing, even when used as a prefix or postfix operator (e.g. `↑`).
    Relation,
    /// Always has some spacing, even when used as a prefix or postfix operator (e.g. `/`).
    Other,
}

/// A stretchable operator.
///
/// It can be created from an `OrdLike` or a `Rel` if the operator is stretchable. This struct
/// carries all the information needed to know how to make the operator stretchy and how to set
/// spacing around it.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct StretchableOp {
    op: MathMLOperator,
    pub stretchy: Stretchy,
    pub spacing: DelimiterSpacing,
}

impl StretchableOp {
    #[inline]
    pub const fn as_op(self) -> MathMLOperator {
        self.op
    }

    /// Creates a `StretchableOp` from an `OrdLike` if it's stretchable. Returns `None` if the
    /// operator isn't stretchable.
    pub const fn from_ord(ord: OrdLike) -> Option<Self> {
        let (stretchy, spacing) = match ord.category() {
            OrdCategory::F | OrdCategory::G => (Stretchy::Always, DelimiterSpacing::Zero),
            OrdCategory::FGandForceDefault => {
                (Stretchy::PrePostfix, DelimiterSpacing::InfixRelation)
            }
            OrdCategory::K => (Stretchy::Never, DelimiterSpacing::Zero),
            OrdCategory::KButUsedToBeB => (Stretchy::Never, DelimiterSpacing::Other),
            OrdCategory::D | OrdCategory::E | OrdCategory::I | OrdCategory::IK => {
                return None;
            }
        };
        Some(StretchableOp {
            op: ord.as_op(),
            stretchy,
            spacing,
        })
    }

    /// Creates a `StretchableOp` from a `Rel` if it's stretchable. Returns `None` if the operator
    /// isn't stretchable.
    pub const fn from_rel(rel: Rel) -> Option<Self> {
        match rel.category() {
            RelCategory::A => Some(StretchableOp {
                op: rel.as_op(),
                stretchy: Stretchy::AlwaysAsymmetric,
                spacing: DelimiterSpacing::Relation,
            }),
            RelCategory::Default => None,
        }
    }
}

/// Creates a fenced expression where opening and closing delimiters are stretched to fit the height
/// of the content. If `open` or `close` is `None`, no delimiter will be rendered on that side.
pub fn fenced<'arena>(
    arena: &'arena Arena,
    mut content: Vec<&'arena Node<'arena>>,
    open: Option<StretchableOp>,
    close: Option<StretchableOp>,
    style: Option<Style>,
) -> Node<'arena> {
    fn to_operator(delim: Option<StretchableOp>) -> Node<'static> {
        if let Some(op) = delim {
            let attrs = if matches!(op.stretchy, Stretchy::Never) {
                OpAttrs::STRETCHY_TRUE
            } else {
                OpAttrs::empty()
            };
            let (left, right) = if matches!(
                op.spacing,
                DelimiterSpacing::Relation | DelimiterSpacing::Other
            ) {
                (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
            } else {
                (None, None)
            };
            Node::Operator {
                op: op.as_op(),
                attrs,
                size: None,
                left,
                right,
            }
        } else {
            // An empty `<mo></mo>` produces weird spacing in some browsers.
            // Use U+2063 (INVISIBLE SEPARATOR) to work around this. It's in Category K in MathML Core.
            Node::Operator {
                op: const { symbol::INVISIBLE_SEPARATOR.as_op() },
                attrs: OpAttrs::empty(),
                size: None,
                left: None,
                right: None,
            }
        }
    }
    let open = arena.push(to_operator(open));
    let close = arena.push(to_operator(close));
    content.insert(0, open);
    content.push(close);
    let nodes = arena.push_slice(&content);
    Node::Row {
        nodes,
        attr: style.map(RowAttr::Style),
    }
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
