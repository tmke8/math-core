use std::num::NonZeroU16;

use mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::Style,
    symbol::{self, StretchableOp},
    table::{Alignment, ArraySpec},
};

static ENVIRONMENTS: phf::Map<&'static str, Env> = phf::phf_map! {
    "array" => Env::Array,
    "subarray" => Env::Subarray,
    "align" => Env::Align,
    "align*" => Env::AlignStar,
    "aligned" => Env::Aligned,
    "equation" => Env::Equation,
    "equation*" => Env::EquationStar,
    "gather" => Env::Gather,
    "gather*" => Env::GatherStar,
    "gathered" => Env::Gathered,
    "multline" => Env::MultLine,
    "bmatrix" => Env::BMatrix,
    "Bmatrix" => Env::Bmatrix,
    "cases" => Env::Cases,
    "matrix" => Env::Matrix,
    "pmatrix" => Env::PMatrix,
    "vmatrix" => Env::VMatrix,
    "Vmatrix" => Env::Vmatrix,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Env {
    Array,
    Subarray,
    Align,
    AlignStar,
    Aligned,
    Equation,
    EquationStar,
    Gather,
    GatherStar,
    Gathered,
    MultLine,
    Cases,
    Matrix,
    BMatrix,
    Bmatrix,
    PMatrix,
    VMatrix,
    Vmatrix,
}

impl Env {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        ENVIRONMENTS.get(s).copied()
    }

    pub(super) fn as_str(&self) -> &'static str {
        ENVIRONMENTS
            .entries()
            .find_map(|(k, v)| if v == self { Some(*k) } else { None })
            .unwrap_or("unknown")
    }

    #[inline]
    pub(super) fn needs_string_literal(&self) -> bool {
        matches!(self, Env::Array | Env::Subarray)
    }

    #[inline]
    pub(super) fn allows_columns(&self) -> bool {
        !matches!(
            self,
            Env::Equation
                | Env::EquationStar
                | Env::Gather
                | Env::GatherStar
                | Env::Gathered
                | Env::MultLine
        )
    }

    #[inline]
    pub(super) fn meaningful_newlines(&self) -> bool {
        !matches!(self, Env::Equation | Env::EquationStar)
    }

    #[inline]
    pub(super) fn get_numbered_env_state(&self) -> Option<NumberedEnvState> {
        if matches!(
            self,
            Env::Align
                | Env::AlignStar
                | Env::Equation
                | Env::EquationStar
                | Env::Gather
                | Env::GatherStar
                | Env::MultLine
        ) {
            Some(NumberedEnvState {
                mode: match self {
                    Env::Align | Env::Equation | Env::Gather => NumberingMode::AllByDefault,
                    Env::MultLine => NumberingMode::OnlyLast,
                    _ => NumberingMode::NoneByDefault,
                },
                num_rows: if matches!(self, Env::MultLine) {
                    NonZeroU16::new(1)
                } else {
                    None
                },
                ..Default::default()
            })
        } else {
            None
        }
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
            Env::Equation | Env::EquationStar | Env::Gather | Env::GatherStar => {
                Node::EquationArray {
                    align: Alignment::Centered,
                    last_equation_num,
                    content,
                }
            }
            Env::Gathered => Node::Table {
                align: Alignment::Centered,
                style: Some(Style::Display),
                content,
            },
            Env::MultLine => {
                debug_assert!(num_rows.is_some());
                Node::MultLine {
                    content,
                    num_rows: num_rows.unwrap_or(NonZeroU16::new(1).unwrap()),
                    last_equation_num,
                }
            }
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
                debug_assert!(array_spec.is_some());
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) enum NumberingMode {
    #[default]
    NoneByDefault,
    AllByDefault,
    OnlyLast,
}

/// State for environments that number equations.
#[derive(Default)]
pub(super) struct NumberedEnvState {
    pub(super) mode: NumberingMode,
    pub(super) suppress_next_number: bool,
    pub(super) custom_next_number: Option<NonZeroU16>,
    pub(super) num_rows: Option<NonZeroU16>,
}

impl NumberedEnvState {
    pub(super) fn next_equation_number(
        &mut self,
        equation_counter: &mut u16,
        is_last: bool,
    ) -> Result<Option<NonZeroU16>, ()> {
        if matches!(self.mode, NumberingMode::OnlyLast) && !is_last {
            // Not the last row; do nothing for now.
            return Ok(None);
        }
        // A custom number takes precedence over suppression.
        if let Some(custom_number) = self.custom_next_number.take() {
            // The state has already been cleared here through `take()`.
            Ok(Some(custom_number))
        } else if self.suppress_next_number || matches!(self.mode, NumberingMode::NoneByDefault) {
            // Clear the flag.
            self.suppress_next_number = false;
            Ok(None)
        } else {
            *equation_counter = equation_counter.checked_add(1).ok_or(())?;
            let equation_number = NonZeroU16::new(*equation_counter);
            debug_assert!(equation_number.is_some());
            Ok(equation_number)
        }
    }
}
