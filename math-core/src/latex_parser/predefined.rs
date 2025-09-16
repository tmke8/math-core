use crate::mathml_renderer::{attribute::MathVariant, symbol};

use super::specifications::LatexUnit;
use super::token::Token::{self, *};

pub static ODV: [Token<'static>; 11] = [
    Frac(None),                     // \frac
    GroupBegin,                     // {
    Transform(MathVariant::Normal), // \mathrm
    Letter('d'),                    // d
    CustomCmdArg(0),                // #1
    GroupEnd,                       // }
    GroupBegin,                     // {
    Transform(MathVariant::Normal), // \mathrm
    Letter('d'),                    // d
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
    TokenStream(0, &XARROW_SPACING_HACK),
    GroupBegin,
    Relation(symbol::RIGHTWARDS_ARROW),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static XLEFTARROW: [Token<'static>; 7] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Overset,
    TokenStream(0, &XARROW_SPACING_HACK),
    GroupBegin,
    Relation(symbol::LEFTWARDS_ARROW),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static DOTS: [Token<'static>; 9] = [
    GroupBegin,
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    BigOp(symbol::FULL_STOP),
    BigOp(symbol::FULL_STOP),
    GroupBegin,
    BigOp(symbol::FULL_STOP),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    GroupEnd,
];

pub static CDOTS: [Token<'static>; 9] = [
    GroupBegin,
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    GroupBegin,
    BinaryOp(symbol::MIDDLE_DOT),
    GroupEnd,
    BinaryOp(symbol::MIDDLE_DOT),
    BinaryOp(symbol::MIDDLE_DOT),
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    GroupEnd,
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

pub static BMOD: [Token<'static>; 8] = [
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    Text(None),
    GroupBegin,
    Letter('m'),
    Letter('o'),
    Letter('d'),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(4.0)),
];

pub static MOD: [Token<'static>; 8] = [
    Space(LatexUnit::Em.length_with_unit(1.0)),
    Text(None),
    GroupBegin,
    Letter('m'),
    Letter('o'),
    Letter('d'),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(6.0)),
];

pub static PMOD: [Token<'static>; 11] = [
    Space(LatexUnit::Em.length_with_unit(1.0)),
    Open(symbol::LEFT_PARENTHESIS),
    Text(None),
    GroupBegin,
    Letter('m'),
    Letter('o'),
    Letter('d'),
    GroupEnd,
    Space(LatexUnit::Mu.length_with_unit(6.0)),
    CustomCmdArg(0),
    Close(symbol::RIGHT_PARENTHESIS),
];
