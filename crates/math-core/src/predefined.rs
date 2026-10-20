use mathml_renderer::{attribute::Notation, symbol};

use crate::token::Operator::*;
use crate::token::Token::{self, *};
use crate::token::{MathChoice, VerticalLineDef as VLDef};
use crate::{character_class::MathVariant, token::PhantomKind};
use crate::{specifications::LatexUnit, token::Mode};

pub static DOTS: [Token; 3] = [
    Operator(Inner(symbol::FULL_STOP)),
    Operator(ForcePunct(symbol::FULL_STOP.as_op())),
    Operator(Inner(symbol::FULL_STOP)),
];

pub static CDOTS: [Token; 3] = [
    Operator(Inner(symbol::MIDDLE_DOT)),
    Operator(ForcePunct(symbol::MIDDLE_DOT.as_op())),
    Operator(Inner(symbol::MIDDLE_DOT)),
];

pub static IDOTSINT: [Token; 3] = [
    OperatorWithLimits(symbol::INTEGRAL),
    CustomCmd(0, &CDOTS),
    OperatorWithLimits(symbol::INTEGRAL),
];

pub static MATHSTRUT: [Token; 2] = [
    Phantom(PhantomKind::V),
    Operator(Open(symbol::LEFT_PARENTHESIS)),
];

pub static SURD: [Token; 3] = [
    Sqrt,
    Phantom(PhantomKind::V),
    Letter(symbol::VERTICAL_LINE.as_bmp_op().as_char(), Mode::Math),
];

pub static AND: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Operator(ForceRel(symbol::AMPERSAND.as_op())),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IFF: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Operator(Rel(symbol::LONG_LEFT_RIGHT_DOUBLE_ARROW)),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIEDBY: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Operator(Rel(symbol::LONG_LEFTWARDS_DOUBLE_ARROW)),
    Space(LatexUnit::Mu.length_with_unit(5.0)),
];

pub static IMPLIES: [Token; 3] = [
    Space(LatexUnit::Mu.length_with_unit(5.0)),
    Operator(Rel(symbol::LONG_RIGHTWARDS_DOUBLE_ARROW)),
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
    Operator(Open(symbol::LEFT_PARENTHESIS)),
    CustomCmdArg(0),
    Operator(Close(symbol::RIGHT_PARENTHESIS)),
];

pub static PMOD: [Token; 7] = [
    MathChoiceInternal(None, &POD_SPACE),
    Operator(Open(symbol::LEFT_PARENTHESIS)),
    Transform(MathVariant::Normal),
    InternalStringLiteral("mod"),
    Space(LatexUnit::Mu.length_with_unit(6.0)),
    CustomCmdArg(0),
    Operator(Close(symbol::RIGHT_PARENTHESIS)),
];

pub static BRA: [Token; 3] = [
    Operator(Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET)),
    CustomCmdArg(0),
    Operator(Close(symbol::VERTICAL_LINE)),
];

pub static KET: [Token; 3] = [
    Operator(Open(symbol::VERTICAL_LINE)),
    CustomCmdArg(0),
    Operator(Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET)),
];

pub static BRAKET: [Token; 3] = [
    Operator(Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET)),
    CustomCmdArg(0),
    Operator(Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET)),
];

pub static BIG_BRA: [Token; 5] = [
    Left,
    Operator(Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET)),
    CustomCmdArg(0),
    Right,
    Operator(Close(symbol::VERTICAL_LINE)),
];

pub static BIG_KET: [Token; 5] = [
    Left,
    Operator(Open(symbol::VERTICAL_LINE)),
    CustomCmdArg(0),
    Right,
    Operator(Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET)),
];

/// `\Braket{...}`, which is `\left\langle ... \right\rangle` with `|` acting as `\,\middle|\,`.
pub static BIG_BRAKET: [Token; 7] = [
    Left,
    Operator(Open(symbol::MATHEMATICAL_LEFT_ANGLE_BRACKET)),
    VerticalLineDef(Some(VLDef::OpSpacingStretchy)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Right,
    Operator(Close(symbol::MATHEMATICAL_RIGHT_ANGLE_BRACKET)),
];

/// `\set{...}`, which is `\{\, ... \,\}` with `|` acting as `\;|\;`.
pub static SET: [Token; 7] = [
    Operator(Open(symbol::LEFT_CURLY_BRACKET)),
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    VerticalLineDef(Some(VLDef::RelSpacing)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Space(LatexUnit::Mu.length_with_unit(3.0)),
    Operator(Close(symbol::RIGHT_CURLY_BRACKET)),
];

/// `\Set{...}`, which is `\left\{\: ... \:\right\}` with `|` acting as `\;\middle|\;`.
pub static BIG_SET: [Token; 9] = [
    Left,
    Operator(Open(symbol::LEFT_CURLY_BRACKET)),
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    VerticalLineDef(Some(VLDef::RelSpacingStretchy)),
    CustomCmdArg(0),
    VerticalLineDef(None), // reset
    Space(LatexUnit::Mu.length_with_unit(4.0)),
    Right,
    Operator(Close(symbol::RIGHT_CURLY_BRACKET)),
];

/// `\fbox{...}`, which is `\boxed{\text{#1}}`.
///
/// The difference to `\boxed` is that the content of an `\fbox` is text rather than math,
/// which is why the argument goes through `\text` here. The braces around it are needed
/// because `\text` takes only a single token otherwise.
pub static FBOX: [Token; 5] = [
    Enclose(Notation::BOX),
    Text(None),
    GroupBegin,
    CustomCmdArg(0),
    GroupEnd,
];

/// `\textcolor{...}{...}`, which is `{\color{#1}#2}`.
///
/// The braces around the first argument matter: `\color` reads the color name as a string
/// literal, so the argument has to be a group, just as it is in the LaTeX definition.
pub static TEXTCOLOR: [Token; 7] = [
    GroupBegin,
    Color,
    GroupBegin,
    CustomCmdArg(0),
    GroupEnd,
    CustomCmdArg(1),
    GroupEnd,
];

pub static COLON_EQUALS: [Token; 2] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::EQUALS_SIGN)),
];

pub static DOUBLE_COLON_EQUAL: [Token; 3] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::EQUALS_SIGN)),
];

pub static EQUALS_COLON: [Token; 2] = [
    Operator(Rel(symbol::EQUALS_SIGN)),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static DOUBLE_COLON: [Token; 2] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static COLON_APPROX: [Token; 2] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::ALMOST_EQUAL_TO)),
];

pub static DOUBLE_COLON_APPROX: [Token; 3] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::ALMOST_EQUAL_TO)),
];

pub static COLON_MINUS: [Token; 2] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::MINUS_SIGN.as_op())),
];

pub static DOUBLE_COLON_MINUS: [Token; 3] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::MINUS_SIGN.as_op())),
];

pub static COLON_SIM: [Token; 2] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::TILDE_OPERATOR)),
];

pub static DOUBLE_COLON_SIM: [Token; 3] = [
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(Rel(symbol::TILDE_OPERATOR)),
];

pub static EQUALS_DOUBLE_COLON: [Token; 3] = [
    Operator(Rel(symbol::EQUALS_SIGN)),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static MINUS_COLON: [Token; 2] = [
    Operator(ForceRel(symbol::MINUS_SIGN.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static MINUS_DOUBLE_COLON: [Token; 3] = [
    Operator(ForceRel(symbol::MINUS_SIGN.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static SIM_COLON: [Token; 2] = [
    Operator(Rel(symbol::TILDE_OPERATOR)),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static SIM_DOUBLE_COLON: [Token; 3] = [
    Operator(Rel(symbol::TILDE_OPERATOR)),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static ANGLN: [Token; 2] = [Enclose(Notation::ACTUARIAL), Letter('n', Mode::Math)];

pub static APPROX_COLON: [Token; 2] = [
    Operator(Rel(symbol::ALMOST_EQUAL_TO)),
    Operator(ForceRel(symbol::RATIO.as_op())),
];

pub static APPROX_DOUBLE_COLON: [Token; 3] = [
    Operator(Rel(symbol::ALMOST_EQUAL_TO)),
    Operator(ForceRel(symbol::RATIO.as_op())),
    Operator(ForceRel(symbol::RATIO.as_op())),
];
