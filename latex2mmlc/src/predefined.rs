use crate::{
    ast::Node,
    attribute::{MathVariant, StretchMode},
    ops,
};

pub static PMOD: Node = Node::RowSlice {
    nodes: &[
        Node::Space("1"),
        Node::StretchableOp(ops::LEFT_PARENTHESIS, StretchMode::NoStretch),
        Node::Text("mod"),
        Node::Space("0.3333"),
        Node::CustomCmdArg(0),
        Node::StretchableOp(ops::RIGHT_PARENTHESIS, StretchMode::NoStretch),
    ],
    style: None,
};

pub static ODV: Node = Node::Frac {
    num: &Node::RowSlice {
        nodes: &[
            Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::SingleLetterIdent('d', false),
            },
            Node::CustomCmdArg(0),
        ],
        style: None,
    },
    den: &Node::RowSlice {
        nodes: &[
            Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::SingleLetterIdent('d', false),
            },
            Node::CustomCmdArg(1),
        ],
        style: None,
    },
    lt: None,
    attr: None,
};
