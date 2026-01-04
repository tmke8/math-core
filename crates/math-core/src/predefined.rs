use mathml_renderer::{attribute::MathVariant, symbol};

use crate::specifications::LatexUnit;
use crate::token::FromAscii;
use crate::token::Token::{self, *};

pub static ODV: [Token<'static>; 11] = [
    Frac(None),                     // \frac
    GroupBegin,                     // {
    Transform(MathVariant::Normal), // \mathrm
    Letter('d', FromAscii::True),   // d
    CustomCmdArg(0),                // #1
    GroupEnd,                       // }
    GroupBegin,                     // {
    Transform(MathVariant::Normal), // \mathrm
    Letter('d', FromAscii::True),   // d
    CustomCmdArg(1),                // #2
    GroupEnd,                       // }
];

static XARROW_SPACING_HACK: [Token<'static>; 7] = [
    Overset,
    Space(LatexUnit::Em.length_with_unit(3.5)),
    GroupBegin,
    Space(LatexUnit::Em.length_with_unit(0.4286)),
    CustomCmdArg(0),
    Space(LatexUnit::Em.length_with_unit(0.4286)),
    GroupEnd,
];

pub static XRIGHTARROW: [Token<'static>; 7] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Overset,
    CustomCmd(0, &XARROW_SPACING_HACK),
    GroupBegin,
    Relation(symbol::RIGHTWARDS_ARROW),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static XLEFTARROW: [Token<'static>; 7] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Overset,
    CustomCmd(0, &XARROW_SPACING_HACK),
    GroupBegin,
    Relation(symbol::LEFTWARDS_ARROW),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

static DOT: &str = "<mo>.</mo>";

pub static DOTS: [Token<'static>; 5] = [
    GroupBegin,
    HardcodedMathML(DOT),
    HardcodedMathML(r#"<mo lspace="0" rspace="0">.</mo>"#),
    HardcodedMathML(DOT),
    GroupEnd,
];

static CDOT: &str = "<mo>·</mo>";
static CDOT_NO_SPACING: &str = r#"<mo lspace="0" rspace="0">·</mo>"#;

pub static CDOTS: [Token<'static>; 5] = [
    GroupBegin,
    HardcodedMathML(CDOT),
    HardcodedMathML(CDOT_NO_SPACING),
    HardcodedMathML(CDOT),
    GroupEnd,
];

pub static IDOTSINT: [Token<'static>; 5] = [
    Integral(symbol::INTEGRAL),
    HardcodedMathML(CDOT_NO_SPACING),
    HardcodedMathML(CDOT),
    HardcodedMathML(CDOT_NO_SPACING),
    Integral(symbol::INTEGRAL),
];

pub static AND: [Token<'static>; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    ForceRelation(symbol::AMPERSAND.as_op()),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IFF: [Token<'static>; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_LEFT_RIGHT_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIEDBY: [Token<'static>; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_LEFTWARDS_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIES: [Token<'static>; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_RIGHTWARDS_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static BMOD: [Token<'static>; 4] = [
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(4.0)),
];

pub static MOD: [Token<'static>; 4] = [
    Space(LatexUnit::Em.length_with_unit(1.0)),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(6.0)),
];

pub static PMOD: [Token<'static>; 7] = [
    Space(LatexUnit::Em.length_with_unit(1.0)),
    Open(symbol::LEFT_PARENTHESIS),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(6.0)),
    CustomCmdArg(0),
    Close(symbol::RIGHT_PARENTHESIS),
];
