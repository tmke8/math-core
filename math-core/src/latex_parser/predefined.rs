use crate::mathml_renderer::{
    ast::Node::{self, *},
    attribute::{LetterAttr, MathSpacing, MathVariant, RowAttr, StretchMode},
    length::{Length, LengthUnit, LengthValue},
    symbol,
};

use super::specifications::LatexUnit;

pub static MOD: Node = Row {
    nodes: &[
        &Space(LatexUnit::Em.length_with_unit(1.0)),
        &Text("mod"),
        &Space(LatexUnit::Mu.length_with_unit(6.0)),
        &CustomCmdArg(0),
    ],
    attr: RowAttr::None,
};

pub static PMOD: Node = Row {
    nodes: &[
        &Space(LatexUnit::Em.length_with_unit(1.0)),
        &StretchableOp(symbol::LEFT_PARENTHESIS, StretchMode::NoStretch),
        &Text("mod"),
        &Space(LatexUnit::Mu.length_with_unit(6.0)),
        &CustomCmdArg(0),
        &StretchableOp(symbol::RIGHT_PARENTHESIS, StretchMode::NoStretch),
    ],
    attr: RowAttr::None,
};

const LENGTH_NONE: (LengthValue, LengthUnit) = Length::none().into_parts();

pub static ODV: Node = Frac {
    num: &Row {
        nodes: &[
            &TextTransform {
                tf: MathVariant::Normal,
                content: &IdentifierChar('d', LetterAttr::Default),
            },
            &CustomCmdArg(0),
        ],
        attr: RowAttr::None,
    },
    denom: &Node::Row {
        nodes: &[
            &TextTransform {
                tf: MathVariant::Normal,
                content: &IdentifierChar('d', LetterAttr::Default),
            },
            &CustomCmdArg(1),
        ],
        attr: RowAttr::None,
    },
    lt_value: LENGTH_NONE.0,
    lt_unit: LENGTH_NONE.1,
    attr: None,
};

static XARROW_SPACING_HACK: Node = Overset {
    target: &Row {
        nodes: &[
            &Space(LatexUnit::Em.length_with_unit(0.4286)),
            &CustomCmdArg(0),
            &Space(LatexUnit::Em.length_with_unit(0.4286)),
        ],
        attr: RowAttr::None,
    },
    symbol: &Space(LatexUnit::Em.length_with_unit(3.5)),
};

pub static XRIGHTARROW: Node = Row {
    nodes: &[
        &Space(LatexUnit::Mu.length_with_unit(5.0)),
        &Overset {
            target: &Operator {
                op: symbol::RIGHTWARDS_ARROW.as_op(),
                attr: None,
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        &Space(LatexUnit::Mu.length_with_unit(5.0)),
    ],
    attr: RowAttr::None,
};

pub static XLEFTARROW: Node = Row {
    nodes: &[
        &Space(LatexUnit::Mu.length_with_unit(5.0)),
        &Overset {
            target: &Operator {
                op: symbol::LEFTWARDS_ARROW.as_op(),
                attr: None,
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        &Space(LatexUnit::Mu.length_with_unit(5.0)),
    ],
    attr: RowAttr::None,
};

pub static DOTS: Node = Row {
    nodes: &[
        &Operator {
            op: symbol::FULL_STOP.as_op(),
            attr: None,
            left: None,
            right: None,
        },
        &Operator {
            op: symbol::FULL_STOP.as_op(),
            attr: None,
            left: Some(MathSpacing::Zero),
            right: Some(MathSpacing::Zero),
        },
        &Operator {
            op: symbol::FULL_STOP.as_op(),
            attr: None,
            left: None,
            right: None,
        },
    ],
    attr: RowAttr::None,
};
