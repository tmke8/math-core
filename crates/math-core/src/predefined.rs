use mathml_renderer::symbol;

use crate::token::Token::{self, *};
use crate::{character_class::MathVariant, token::PhantomKind};
use crate::{specifications::LatexUnit, token::Mode};

pub static DOTS: [Token<'static>; 3] = [
    Inner(symbol::FULL_STOP),
    ForcePunctuation(symbol::FULL_STOP.as_op()),
    Inner(symbol::FULL_STOP),
];

pub static CDOTS: [Token<'static>; 3] = [
    Inner(symbol::MIDDLE_DOT),
    ForcePunctuation(symbol::MIDDLE_DOT.as_op()),
    Inner(symbol::MIDDLE_DOT),
];

pub static IDOTSINT: [Token<'static>; 3] = [
    Op(symbol::INTEGRAL),
    CustomCmd(0, &CDOTS),
    Op(symbol::INTEGRAL),
];

pub static MATHSTRUT: [Token<'static>; 2] =
    [Phantom(PhantomKind::V), Open(symbol::LEFT_PARENTHESIS)];

pub static SURD: [Token<'static>; 3] = [
    Sqrt,
    Phantom(PhantomKind::V),
    Letter(symbol::VERTICAL_LINE.as_op().as_superchar(), Mode::Math),
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

pub static BRA: [Token<'static>; 3] = [
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Close(symbol::VERTICAL_LINE),
];

pub static KET: [Token<'static>; 3] = [
    Open(symbol::VERTICAL_LINE),
    CustomCmdArg(0),
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

pub static BRAKET: [Token<'static>; 3] = [
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

pub static BIG_BRA: [Token<'static>; 5] = [
    Left,
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Right,
    Close(symbol::VERTICAL_LINE),
];

pub static BIG_KET: [Token<'static>; 5] = [
    Left,
    Open(symbol::VERTICAL_LINE),
    CustomCmdArg(0),
    Right,
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

pub static COLON_EQUALS: [Token<'static>; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::EQUALS_SIGN),
];

pub static DOUBLE_COLON_EQUAL: [Token<'static>; 3] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::EQUALS_SIGN),
];

pub static EQUALS_COLON: [Token<'static>; 2] = [
    Relation(symbol::EQUALS_SIGN),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static DOUBLE_COLON: [Token<'static>; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];
