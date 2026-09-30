use alloc::vec::Vec;

use mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::{MathSpacing, OpAttrs, OpRoles, RowAttrs, Style, TextTransform},
    symbol::{self, MathMLOperator, OrdCategory, OrdLike, Rel, RelCategory},
};

use crate::token::ForceStretchy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParenType {
    Left = 1,
    Right,
    Middle,
}

/// <mi> mathvariant attribute
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathVariant {
    /// This is enforced by setting `mathvariant="normal"`.
    Normal,
    /// This is enforced by transforming the characters themselves.
    Transform(TextTransform),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stretchy {
    /// The operator is always stretchy (e.g. `(`, `)`).
    /// (Or, it's [`ForceStretchy::Pretend`].)
    Always = 1,
    /// The operator is only stretchy as a pre- or postfix operator (e.g. `|`).
    PrePostfix,
    /// The operator is never stretchy (e.g. `/`).
    Never,
    /// The operator is always stretchy but isn't symmetric (e.g. `↑`).
    AlwaysAsymmetric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
            OrdCategory::F | OrdCategory::G | OrdCategory::FG => {
                (Stretchy::Always, DelimiterSpacing::Zero)
            }
            OrdCategory::FGandForceDefault => {
                (Stretchy::PrePostfix, DelimiterSpacing::InfixRelation)
            }
            OrdCategory::K => (Stretchy::Never, DelimiterSpacing::Zero),
            OrdCategory::KButUsedToBeB => (Stretchy::Never, DelimiterSpacing::Other),
            OrdCategory::D
            | OrdCategory::E
            | OrdCategory::DE
            | OrdCategory::I
            | OrdCategory::IK => {
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
            RelCategory::Default | RelCategory::DandForceDefault => None,
        }
    }

    /// Creates a `StretchableOp` from a [`ForceStretchy`] if it's stretchable.
    /// Used with `ForceOpen`/`ForceClose`.
    pub const fn from_force_stretchy(op: MathMLOperator, stretch: ForceStretchy) -> Option<Self> {
        match stretch {
            ForceStretchy::Yes => Some(StretchableOp {
                op,
                stretchy: Stretchy::Never,
                spacing: DelimiterSpacing::Zero,
            }),
            ForceStretchy::Pretend => Some(StretchableOp {
                op,
                stretchy: Stretchy::Always,
                spacing: DelimiterSpacing::Zero,
            }),
            ForceStretchy::No => None,
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
                roles: OpRoles::empty(),
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
                roles: OpRoles::empty(),
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
        attrs: RowAttrs {
            style,
            ..RowAttrs::DEFAULT
        },
    }
}

/// Adds spacing outside of a fence that was created with [`fenced`].
///
/// The spacing is realized as `lspace` on the opening delimiter and `rspace` on the closing
/// delimiter. (If a side has no delimiter, the invisible placeholder operator carries it.)
pub fn with_outer_spacing<'arena>(
    arena: &'arena Arena,
    fence: Node<'arena>,
    outer_left: Option<MathSpacing>,
    outer_right: Option<MathSpacing>,
) -> Node<'arena> {
    let Node::Row { nodes, attrs } = fence else {
        debug_assert!(false, "expected a fence created by `fenced`");
        return fence;
    };
    let [first, .., last] = nodes else {
        debug_assert!(false, "expected a fence created by `fenced`");
        return Node::Row { nodes, attrs };
    };
    let with_spacing = |node: &'arena Node<'arena>,
                        outer_left: Option<MathSpacing>,
                        outer_right: Option<MathSpacing>| {
        let Node::Operator {
            op,
            attrs,
            roles,
            size,
            left,
            right,
        } = *node
        else {
            debug_assert!(false, "expected a delimiter");
            return node;
        };
        arena.push(Node::Operator {
            op,
            attrs,
            roles,
            size,
            left: outer_left.or(left),
            right: outer_right.or(right),
        })
    };
    let mut new_nodes = nodes.to_vec();
    if outer_left.is_some() {
        new_nodes[0] = with_spacing(first, outer_left, None);
    }
    if outer_right.is_some() {
        new_nodes[nodes.len() - 1] = with_spacing(last, None, outer_right);
    }
    Node::Row {
        nodes: arena.push_slice(&new_nodes),
        attrs,
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
