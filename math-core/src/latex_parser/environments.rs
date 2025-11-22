use std::num::NonZeroU16;

use strum_macros::IntoStaticStr;

use crate::mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::Style,
    symbol::{self, StretchableOp},
    table::{Alignment, ArraySpec},
};

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
pub enum Env {
    #[strum(serialize = "array")]
    Array,
    #[strum(serialize = "subarray")]
    Subarray,
    #[strum(serialize = "align")]
    Align,
    #[strum(serialize = "align*")]
    AlignStar,
    #[strum(serialize = "aligned")]
    Aligned,
    #[strum(serialize = "multline")]
    MultLine,
    #[strum(serialize = "cases")]
    Cases,
    #[strum(serialize = "matrix")]
    Matrix,
    #[strum(serialize = "bmatrix")]
    BMatrix,
    #[strum(serialize = "Bmatrix")]
    Bmatrix,
    #[strum(serialize = "pmatrix")]
    PMatrix,
    #[strum(serialize = "vmatrix")]
    VMatrix,
    #[strum(serialize = "Vmatrix")]
    Vmatrix,
}

impl Env {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        ENVIRONMENTS.get(s).copied()
    }

    #[inline]
    pub(super) fn needs_string_literal(&self) -> bool {
        matches!(self, Env::Array | Env::Subarray)
    }

    pub(super) fn construct_node<'arena>(
        &self,
        content: &'arena [&'arena Node<'arena>],
        array_spec: Option<&'arena ArraySpec<'arena>>,
        arena: &'arena Arena,
        last_equation_num: Option<NonZeroU16>,
        num_rows: Option<NonZeroU16>,
    ) -> Node<'arena> {
        match self {
            Env::Align | Env::AlignStar => Node::EquationArray {
                align: Alignment::Alternating,
                last_equation_num,
                content,
            },
            Env::Aligned => Node::Table {
                align: Alignment::Alternating,
                style: Some(Style::Display),
                content,
            },
            Env::MultLine => Node::MultLine {
                content,
                num_rows: num_rows.unwrap_or(NonZeroU16::new(1).unwrap()),
                last_equation_num,
            },
            Env::Cases => {
                let align = Alignment::Cases;
                let content = arena.push(Node::Table {
                    content,
                    align,
                    style: None,
                });
                Node::Fenced {
                    open: Some(symbol::LEFT_CURLY_BRACKET.as_op()),
                    close: None,
                    content,
                    style: None,
                }
            }
            Env::Matrix => Node::Table {
                align: Alignment::Centered,
                style: Some(Style::Display),
                content,
            },
            array_variant @ (Env::Array | Env::Subarray) => {
                // SAFETY: `array_spec` is guaranteed to be Some because we checked for
                // `Env::Array` and `Env::Subarray` in the caller.
                // TODO: Refactor this to avoid using `unsafe`.
                let array_spec = unsafe { array_spec.unwrap_unchecked() };
                let style = if matches!(array_variant, Env::Subarray) {
                    Some(Style::Script)
                } else {
                    None
                };
                Node::Array {
                    style,
                    content,
                    array_spec,
                }
            }
            matrix_variant @ (Env::PMatrix
            | Env::BMatrix
            | Env::Bmatrix
            | Env::VMatrix
            | Env::Vmatrix) => {
                let align = Alignment::Centered;
                let (open, close) = match matrix_variant {
                    Env::PMatrix => (
                        symbol::LEFT_PARENTHESIS.as_op(),
                        symbol::RIGHT_PARENTHESIS.as_op(),
                    ),
                    Env::BMatrix => (
                        symbol::LEFT_SQUARE_BRACKET.as_op(),
                        symbol::RIGHT_SQUARE_BRACKET.as_op(),
                    ),
                    Env::Bmatrix => (
                        symbol::LEFT_CURLY_BRACKET.as_op(),
                        symbol::RIGHT_CURLY_BRACKET.as_op(),
                    ),
                    Env::VMatrix => {
                        const LINE: StretchableOp =
                            symbol::VERTICAL_LINE.as_stretchable_op().unwrap();
                        (LINE, LINE)
                    }
                    Env::Vmatrix => {
                        const DOUBLE_LINE: StretchableOp =
                            symbol::DOUBLE_VERTICAL_LINE.as_stretchable_op().unwrap();
                        (DOUBLE_LINE, DOUBLE_LINE)
                    }
                    // SAFETY: `matrix_variant` is one of the strings above.
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };
                let style = Some(Style::Display);
                Node::Fenced {
                    open: Some(open),
                    close: Some(close),
                    content: arena.push(Node::Table {
                        content,
                        align,
                        style,
                    }),
                    style: None,
                }
            }
        }
    }
}

static ENVIRONMENTS: phf::Map<&'static str, Env> = phf::phf_map! {
    "array" => Env::Array,
    "subarray" => Env::Subarray,
    "align" => Env::Align,
    "align*" => Env::AlignStar,
    "aligned" => Env::Aligned,
    "multline" => Env::MultLine,
    "bmatrix" => Env::BMatrix,
    "Bmatrix" => Env::Bmatrix,
    "cases" => Env::Cases,
    "matrix" => Env::Matrix,
    "pmatrix" => Env::PMatrix,
    "vmatrix" => Env::VMatrix,
    "Vmatrix" => Env::Vmatrix,
};
