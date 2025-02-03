use crate::{
    arena::{NodeList, NodeListElement},
    ast::Node,
    attribute::StretchMode,
    ops,
};

pub mod pmod {
    use super::*;

    static ELEM6: NodeListElement = NodeListElement::new(Node::StretchableOp(
        ops::RIGHT_PARENTHESIS,
        StretchMode::NoStretch,
    ));
    static ELEM5: NodeListElement = unsafe { NodeListElement::from_raw(Node::FirstArg, &ELEM6) };
    static ELEM4: NodeListElement =
        unsafe { NodeListElement::from_raw(Node::Space("0.3333"), &ELEM5) };
    static ELEM3: NodeListElement = unsafe { NodeListElement::from_raw(Node::Text("mod"), &ELEM4) };
    static ELEM2: NodeListElement = unsafe {
        NodeListElement::from_raw(
            Node::StretchableOp(
                ops::LEFT_PARENTHESIS,
                crate::attribute::StretchMode::NoStretch,
            ),
            &ELEM3,
        )
    };
    static ELEM1: NodeListElement = unsafe { NodeListElement::from_raw(Node::Space("1"), &ELEM2) };
    pub static NODE: Node = Node::Row {
        nodes: unsafe { NodeList::from_raw(&ELEM1) },
        style: None,
    };
}
