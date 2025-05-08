use mathml_renderer::{
    ast::Node::{self, *},
    attribute::{MathSpacing, MathVariant, RowAttr, StretchMode},
    length::{Length, LengthUnit, LengthValue},
    symbol,
};

pub static MOD: Node = Row {
    nodes: &[
        &Space("1"),
        &Text("mod"),
        &Space("0.3333"),
        &CustomCmdArg(0),
    ],
    attr: RowAttr::None,
};

pub static PMOD: Node = Row {
    nodes: &[
        &Space("1"),
        &StretchableOp(symbol::LEFT_PARENTHESIS, StretchMode::NoStretch),
        &Text("mod"),
        &Space("0.3333"),
        &CustomCmdArg(0),
        &StretchableOp(symbol::RIGHT_PARENTHESIS, StretchMode::NoStretch),
    ],
    attr: RowAttr::None,
};

const LENGTH_NONE: (LengthValue, LengthUnit) = Length::empty().into_parts();

pub static ODV: Node = Frac {
    num: &Row {
        nodes: &[
            &TextTransform {
                tf: MathVariant::Normal,
                content: &SingleLetterIdent('d', false),
            },
            &CustomCmdArg(0),
        ],
        attr: RowAttr::None,
    },
    denom: &Node::Row {
        nodes: &[
            &TextTransform {
                tf: MathVariant::Normal,
                content: &SingleLetterIdent('d', false),
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
        nodes: &[&Space("0.4286"), &CustomCmdArg(0), &Space("0.4286")],
        attr: RowAttr::None,
    },
    symbol: &Space("3.5"),
};

pub static XRIGHTARROW: Node = Row {
    nodes: &[
        &Space("0.2778"),
        &Overset {
            target: &OperatorWithSpacing {
                op: symbol::RIGHTWARDS_ARROW.as_op(),
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        &Space("0.2778"),
    ],
    attr: RowAttr::None,
};

pub static XLEFTARROW: Node = Row {
    nodes: &[
        &Space("0.2778"),
        &Overset {
            target: &OperatorWithSpacing {
                op: symbol::LEFTWARDS_ARROW.as_op(),
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        &Space("0.2778"),
    ],
    attr: RowAttr::None,
};
