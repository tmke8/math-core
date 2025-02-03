use crate::{
    arena::{NodeList, NodeListElement},
    ast::Node,
    attribute::StretchMode,
    ops,
};

pub static PMOD: Node = const {
    let nodes = [
        &mut NodeListElement::new(Node::Space("1")),
        &mut NodeListElement::new(Node::StretchableOp(
            ops::LEFT_PARENTHESIS,
            StretchMode::NoStretch,
        )),
        &mut NodeListElement::new(Node::Text("mod")),
        &mut NodeListElement::new(Node::Space("0.3333")),
        &mut NodeListElement::new(Node::FirstArg),
    ];
    let last_element = &mut NodeListElement::new(Node::StretchableOp(
        ops::RIGHT_PARENTHESIS,
        StretchMode::NoStretch,
    ));
    let nodes = NodeList::from_node_refs(nodes, last_element);
    Node::Row { nodes, style: None }
};
