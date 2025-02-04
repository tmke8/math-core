use crate::{ast::Node, attribute::StretchMode, ops};

pub static PMOD: Node = Node::RowSlice {
    nodes: &[
        Node::Space("1"),
        Node::StretchableOp(ops::LEFT_PARENTHESIS, StretchMode::NoStretch),
        Node::Text("mod"),
        Node::Space("0.3333"),
        Node::FirstArg,
        Node::StretchableOp(ops::RIGHT_PARENTHESIS, StretchMode::NoStretch),
    ],
    style: None,
};
