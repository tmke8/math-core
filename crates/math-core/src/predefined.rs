use mathml_renderer::{attribute::Notation, super_char::SuperChar, symbol};

use crate::token::MathChoice;
use crate::token::Token::{self, *};
use crate::token::VerticalLineDef as VLDef;
use crate::{character_class::MathVariant, token::PhantomKind};
use crate::{specifications::LatexUnit, token::Mode};

pub static DOTS: [Token; 3] = [
    Inner(symbol::FULL_STOP),
    ForcePunctuation(symbol::FULL_STOP.as_op()),
    Inner(symbol::FULL_STOP),
];

pub static CDOTS: [Token; 3] = [
    Inner(symbol::MIDDLE_DOT),
    ForcePunctuation(symbol::MIDDLE_DOT.as_op()),
    Inner(symbol::MIDDLE_DOT),
];

pub static IDOTSINT: [Token; 3] = [
    Op(symbol::INTEGRAL),
    CustomCmd(0, &CDOTS),
    Op(symbol::INTEGRAL),
];

pub static MATHSTRUT: [Token; 2] = [Phantom(PhantomKind::V), Open(symbol::LEFT_PARENTHESIS)];

pub static SURD: [Token; 3] = [
    Sqrt,
    Phantom(PhantomKind::V),
    Letter(symbol::VERTICAL_LINE.as_op().as_superchar(), Mode::Math),
];

pub static AND: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    ForceRelation(symbol::AMPERSAND.as_op()),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IFF: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_LEFT_RIGHT_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIEDBY: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_LEFTWARDS_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIES: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Relation(symbol::LONG_RIGHTWARDS_DOUBLE_ARROW),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static BMOD: [Token; 4] = [
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(4.0)),
];

/// The space in front of `\mod`: `\mkern18mu` in display style and `\mkern12mu` otherwise.
static MOD_SPACE: MathChoice = MathChoice {
    display: Space(LatexUnit::Mu.length_with_unit(18.0)),
    text: Space(LatexUnit::Mu.length_with_unit(12.0)),
    script: Space(LatexUnit::Mu.length_with_unit(12.0)),
    scriptscript: Space(LatexUnit::Mu.length_with_unit(12.0)),
};

/// The space in front of `\pod` and `\pmod`: `\mkern18mu` in display style and `\mkern8mu`
/// otherwise.
static POD_SPACE: MathChoice = MathChoice {
    display: Space(LatexUnit::Mu.length_with_unit(18.0)),
    text: Space(LatexUnit::Mu.length_with_unit(8.0)),
    script: Space(LatexUnit::Mu.length_with_unit(8.0)),
    scriptscript: Space(LatexUnit::Mu.length_with_unit(8.0)),
};

pub static MOD: [Token; 4] = [
    MathChoiceInternal(None, &MOD_SPACE),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(6.0)),
];

pub static POD: [Token; 4] = [
    MathChoiceInternal(None, &POD_SPACE),
    Open(symbol::LEFT_PARENTHESIS),
    CustomCmdArg(0),
    Close(symbol::RIGHT_PARENTHESIS),
];

pub static PMOD: [Token; 7] = [
    MathChoiceInternal(None, &POD_SPACE),
    Open(symbol::LEFT_PARENTHESIS),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(6.0)),
    CustomCmdArg(0),
    Close(symbol::RIGHT_PARENTHESIS),
];

pub static BRA: [Token; 3] = [
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Close(symbol::VERTICAL_LINE),
];

pub static KET: [Token; 3] = [
    Open(symbol::VERTICAL_LINE),
    CustomCmdArg(0),
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

pub static BRAKET: [Token; 3] = [
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

pub static BIG_BRA: [Token; 5] = [
    Left,
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    CustomCmdArg(0),
    Right,
    Close(symbol::VERTICAL_LINE),
];

pub static BIG_KET: [Token; 5] = [
    Left,
    Open(symbol::VERTICAL_LINE),
    CustomCmdArg(0),
    Right,
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

/// `\Braket{...}`, which is `\left\langle ... \right\rangle` with `|` acting as `\,\middle|\,`.
pub static BIG_BRAKET: [Token; 7] = [
    Left,
    Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET),
    VerticalLineDef(Some(VLDef::OpSpacingStretchy)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Right,
    Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET),
];

/// `\set{...}`, which is `\{\, ... \,\}` with `|` acting as `\;|\;`.
pub static SET: [Token; 7] = [
    Open(symbol::LEFT_CURLY_BRACKET),
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    VerticalLineDef(Some(VLDef::RelSpacing)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    Close(symbol::RIGHT_CURLY_BRACKET),
];

/// `\Set{...}`, which is `\left\{\: ... \:\right\}` with `|` acting as `\;\middle|\;`.
pub static BIG_SET: [Token; 9] = [
    Left,
    Open(symbol::LEFT_CURLY_BRACKET),
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    VerticalLineDef(Some(VLDef::RelSpacingStretchy)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    Right,
    Close(symbol::RIGHT_CURLY_BRACKET),
];

/// `\textcolor{...}{...}`, which is `{\color{#1}#2}`.
pub static TEXTCOLOR: [Token; 5] = [
    GroupBegin,
    Color,
    CustomCmdArg(0),
    CustomCmdArg(1),
    GroupEnd,
];

pub static COLON_EQUALS: [Token; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::EQUALS_SIGN),
];

pub static DOUBLE_COLON_EQUAL: [Token; 3] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::EQUALS_SIGN),
];

pub static EQUALS_COLON: [Token; 2] = [
    Relation(symbol::EQUALS_SIGN),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static DOUBLE_COLON: [Token; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static COLON_APPROX: [Token; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::ALMOST_EQUAL_TO),
];

pub static DOUBLE_COLON_APPROX: [Token; 3] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::ALMOST_EQUAL_TO),
];

pub static COLON_MINUS: [Token; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::MINUS_SIGN.as_op()),
];

pub static DOUBLE_COLON_MINUS: [Token; 3] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::MINUS_SIGN.as_op()),
];

pub static COLON_SIM: [Token; 2] = [
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::TILDE_OPERATOR),
];

pub static DOUBLE_COLON_SIM: [Token; 3] = [
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    Relation(symbol::TILDE_OPERATOR),
];

pub static EQUALS_DOUBLE_COLON: [Token; 3] = [
    Relation(symbol::EQUALS_SIGN),
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static MINUS_COLON: [Token; 2] = [
    ForceRelation(symbol::MINUS_SIGN.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static MINUS_DOUBLE_COLON: [Token; 3] = [
    ForceRelation(symbol::MINUS_SIGN.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static SIM_COLON: [Token; 2] = [
    Relation(symbol::TILDE_OPERATOR),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static SIM_DOUBLE_COLON: [Token; 3] = [
    Relation(symbol::TILDE_OPERATOR),
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static ANGLN: [Token; 2] = [
    Enclose(Notation::ACTUARIAL),
    Letter(SuperChar::from_char('n'), Mode::Math),
];

pub static APPROX_COLON: [Token; 2] = [
    Relation(symbol::ALMOST_EQUAL_TO),
    ForceRelation(symbol::RATIO.as_op()),
];

pub static APPROX_DOUBLE_COLON: [Token; 3] = [
    Relation(symbol::ALMOST_EQUAL_TO),
    ForceRelation(symbol::RATIO.as_op()),
    ForceRelation(symbol::RATIO.as_op()),
];
