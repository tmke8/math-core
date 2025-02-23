use mathml_renderer::{
    ast::Node::{self, *},
    attribute::{MathSpacing, MathVariant, StretchMode},
    ops,
};

pub static MOD: Node = Row {
    nodes: &[Space("1"), Text("mod"), Space("0.3333"), CustomCmdArg(0)],
    style: None,
};

pub static PMOD: Node = Row {
    nodes: &[
        Space("1"),
        StretchableOp(ops::LEFT_PARENTHESIS, StretchMode::NoStretch),
        Text("mod"),
        Space("0.3333"),
        CustomCmdArg(0),
        StretchableOp(ops::RIGHT_PARENTHESIS, StretchMode::NoStretch),
    ],
    style: None,
};

pub static ODV: Node = Frac {
    num: &Row {
        nodes: &[
            TextTransform {
                tf: MathVariant::Normal,
                content: &SingleLetterIdent('d', false),
            },
            CustomCmdArg(0),
        ],
        style: None,
    },
    den: &Node::Row {
        nodes: &[
            TextTransform {
                tf: MathVariant::Normal,
                content: &SingleLetterIdent('d', false),
            },
            CustomCmdArg(1),
        ],
        style: None,
    },
    lt: None,
    attr: None,
};

static XARROW_SPACING_HACK: Node = Overset {
    target: &Row {
        nodes: &[Space("0.4286"), CustomCmdArg(0), Space("0.4286")],
        style: None,
    },
    symbol: &Space("3.5"),
};

pub static XRIGHTARROW: Node = Row {
    nodes: &[
        Space("0.2778"),
        Overset {
            target: &OperatorWithSpacing {
                op: ops::RIGHTWARDS_ARROW,
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        Space("0.2778"),
    ],
    style: None,
};

pub static XLEFTARROW: Node = Row {
    nodes: &[
        Space("0.2778"),
        Overset {
            target: &OperatorWithSpacing {
                op: ops::LEFTWARDS_ARROW,
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            },
            symbol: &XARROW_SPACING_HACK,
        },
        Space("0.2778"),
    ],
    style: None,
};
