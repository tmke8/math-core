use alloc::string::String;
use core::fmt::Write;
use core::num::NonZeroU16;

use mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::Style,
    symbol,
    table::{Alignment, ArraySpec, ColumnAlignment, EquationTag, LineType, RowLabelInfo},
};

use crate::character_class::StretchableOp;

static ENVIRONMENTS: phf::Map<&'static str, Env> = phf::phf_map! {
    "array" => Env::Array,
    "subarray" => Env::Subarray,
    "align" => Env::Align,
    "align*" => Env::AlignStar,
    "aligned" => Env::Aligned,
    "darray" => Env::DArray,
    "equation" => Env::Equation,
    "equation*" => Env::EquationStar,
    "gather" => Env::Gather,
    "gather*" => Env::GatherStar,
    "gathered" => Env::Gathered,
    "multline" => Env::MultLine,
    "multline*" => Env::MultLineStar,
    "bmatrix" => Env::BMatrix,
    "Bmatrix" => Env::Bmatrix,
    "cases" => Env::Cases,
    "rcases" => Env::RCases,
    "dcases" => Env::DCases,
    "drcases" => Env::DRCases,
    "matrix" => Env::Matrix,
    "pmatrix" => Env::PMatrix,
    "vmatrix" => Env::VMatrix,
    "Vmatrix" => Env::Vmatrix,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    Array,
    DArray,
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
    MultLineStar,
    Cases,
    RCases,
    DCases,
    DRCases,
    Matrix,
    BMatrix,
    Bmatrix,
    PMatrix,
    VMatrix,
    Vmatrix,
}
pub const OPEN_PAREN: StretchableOp = StretchableOp::from_ord(symbol::LEFT_PARENTHESIS).unwrap();
pub const CLOSE_PAREN: StretchableOp = StretchableOp::from_ord(symbol::RIGHT_PARENTHESIS).unwrap();
pub const OPEN_BRACKET: StretchableOp =
    StretchableOp::from_ord(symbol::LEFT_SQUARE_BRACKET).unwrap();
pub const CLOSE_BRACKET: StretchableOp =
    StretchableOp::from_ord(symbol::RIGHT_SQUARE_BRACKET).unwrap();
pub const OPEN_BRACE: StretchableOp = StretchableOp::from_ord(symbol::LEFT_CURLY_BRACKET).unwrap();
pub const CLOSE_BRACE: StretchableOp =
    StretchableOp::from_ord(symbol::RIGHT_CURLY_BRACKET).unwrap();

impl Env {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        ENVIRONMENTS.get(s).copied()
    }

    /// The delimiters that the environment is wrapped in with `\left...\right`,
    /// which makes it a `mathinner` atom. A side without a delimiter is `None`.
    pub(super) const fn delimiters(self) -> Option<(Option<StretchableOp>, Option<StretchableOp>)> {
        const LINE: StretchableOp = StretchableOp::from_ord(symbol::VERTICAL_LINE).unwrap();
        const DOUBLE_LINE: StretchableOp =
            StretchableOp::from_ord(symbol::DOUBLE_VERTICAL_LINE).unwrap();
        Some(match self {
            Env::Cases | Env::DCases => (Some(OPEN_BRACE), None),
            Env::RCases | Env::DRCases => (None, Some(CLOSE_BRACE)),
            Env::PMatrix => (Some(OPEN_PAREN), Some(CLOSE_PAREN)),
            Env::BMatrix => (Some(OPEN_BRACKET), Some(CLOSE_BRACKET)),
            Env::Bmatrix => (Some(OPEN_BRACE), Some(CLOSE_BRACE)),
            Env::VMatrix => (Some(LINE), Some(LINE)),
            Env::Vmatrix => (Some(DOUBLE_LINE), Some(DOUBLE_LINE)),
            _ => return None,
        })
    }

    pub(super) fn as_str(self) -> &'static str {
        ENVIRONMENTS
            .entries()
            .find_map(|(k, v)| if v == &self { Some(*k) } else { None })
            .unwrap_or("unknown")
    }

    #[inline]
    fn allows_columns(self) -> bool {
        !matches!(
            self,
            Env::Equation
                | Env::EquationStar
                | Env::Gather
                | Env::GatherStar
                | Env::Gathered
                | Env::MultLine
                | Env::MultLineStar
        )
    }

    /// `true` for environments in which `\hline`/`\hdashline` may appear (the `array` family and
    /// the `matrix` family).
    #[inline]
    pub(super) fn allows_hlines(self) -> bool {
        matches!(
            self,
            Env::Array
                | Env::DArray
                | Env::Subarray
                | Env::Matrix
                | Env::PMatrix
                | Env::BMatrix
                | Env::Bmatrix
                | Env::VMatrix
                | Env::Vmatrix
        )
    }

    /// `true` for environments in which `\shoveleft`/`\shoveright` may appear.
    #[inline]
    pub(super) fn allows_shove(self) -> bool {
        matches!(self, Env::MultLine | Env::MultLineStar)
    }

    #[inline]
    fn get_numbered_env_state(self) -> Option<NumberedEnvState<'static>> {
        if matches!(
            self,
            Env::Align
                | Env::AlignStar
                | Env::Equation
                | Env::EquationStar
                | Env::Gather
                | Env::GatherStar
                | Env::MultLine
                | Env::MultLineStar
        ) {
            Some(NumberedEnvState {
                mode: match self {
                    Env::Align | Env::Equation | Env::Gather => NumberingMode::AllByDefault,
                    Env::MultLine => NumberingMode::OnlyLast,
                    _ => NumberingMode::NoneByDefault,
                },
                num_rows: if matches!(self, Env::MultLine | Env::MultLineStar) {
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

    pub(super) fn new_state(self) -> EnvState<'static> {
        EnvState {
            allow_columns: self.allows_columns(),
            nested: false,
            meaningful_newlines: !matches!(self, Env::Equation | Env::EquationStar),
            allow_hlines: self.allows_hlines(),
            allow_shove: self.allows_shove(),
            numbered: self.get_numbered_env_state(),
        }
    }

    pub(super) fn style(self) -> Style {
        use Env::*;
        match self {
            DArray | Align | AlignStar | Aligned | Equation | EquationStar | Gather
            | GatherStar | Gathered | MultLine | MultLineStar | DCases | DRCases => Style::Display,
            Array | Cases | RCases | Matrix | BMatrix | Bmatrix | PMatrix | VMatrix | Vmatrix => {
                Style::Text
            }
            Subarray => Style::Script,
        }
    }

    pub(super) fn construct_node<'arena>(
        self,
        content: &'arena [&'arena Node<'arena>],
        array_spec: Option<&'arena ArraySpec<'arena>>,
        last_row_info: Option<&'arena RowLabelInfo<'arena>>,
        num_rows: Option<NonZeroU16>,
        border_top: Option<LineType>,
        initial_shove: Option<ColumnAlignment>,
    ) -> Node<'arena> {
        match self {
            Env::Align | Env::AlignStar => Node::EquationArray {
                align: Alignment::Alternating,
                last_row_info,
                content,
            },
            Env::Aligned => Node::Table {
                align: Alignment::Alternating,
                style: Some(Style::Display),
                content,
                border_top: None,
            },
            Env::Equation | Env::EquationStar | Env::Gather | Env::GatherStar => {
                Node::EquationArray {
                    align: Alignment::Centered,
                    last_row_info,
                    content,
                }
            }
            Env::Gathered => Node::Table {
                align: Alignment::Centered,
                style: Some(Style::Display),
                content,
                border_top: None,
            },
            Env::Matrix => Node::Table {
                align: Alignment::Centered,
                style: Some(Style::Text),
                content,
                border_top,
            },
            Env::MultLine | Env::MultLineStar => {
                debug_assert!(num_rows.is_some());
                Node::MultLine {
                    content,
                    num_rows: num_rows.unwrap_or(NonZeroU16::new(1).unwrap()),
                    last_row_info,
                    initial_shove,
                }
            }
            Env::Cases | Env::RCases => Node::Table {
                content,
                align: Alignment::Cases,
                style: Some(Style::Text),
                border_top: None,
            },
            Env::DCases | Env::DRCases => Node::Table {
                content,
                align: Alignment::Cases,
                style: Some(Style::Display),
                border_top: None,
            },
            array_variant @ (Env::Array | Env::DArray | Env::Subarray) => {
                // SAFETY: `array_spec` is guaranteed to be Some because we checked for
                // `Env::Array`, `Env:DArray` and `Env::Subarray` in the caller.
                // FIXME: Refactor this to avoid using `unsafe`.
                debug_assert!(array_spec.is_some());
                let array_spec = unsafe { array_spec.unwrap_unchecked() };
                let style = match array_variant {
                    Env::Array => Some(Style::Text),
                    Env::DArray => Some(Style::Display),
                    Env::Subarray => Some(Style::Script),
                    _ => unreachable!(),
                };
                Node::Array {
                    style,
                    content,
                    array_spec,
                }
            }
            Env::PMatrix | Env::BMatrix | Env::Bmatrix | Env::VMatrix | Env::Vmatrix => {
                Node::Table {
                    content,
                    align: Alignment::Centered,
                    style: Some(Style::Text),
                    border_top,
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct EnvState<'arena> {
    /// `true` if we are inside an environment that allows columns (`&`).
    pub allow_columns: bool,
    /// `true` if we are inside a group (like `{...}` or `\left...\right`) within the
    /// environment, where `&` and `\\` cannot be used.
    pub nested: bool,
    /// `true` if we should treat newlines as meaningful (i.e., in `align` environments).
    pub meaningful_newlines: bool,
    /// `true` if we are inside an environment where `\hline` and `\hdashline` are allowed
    /// (directly after `\\` or at the start of the environment).
    pub allow_hlines: bool,
    /// `true` if we are inside an environment where `\shoveleft` and `\shoveright` are allowed
    /// (directly after `\\` or at the start of the environment).
    pub allow_shove: bool,
    pub numbered: Option<NumberedEnvState<'arena>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum NumberingMode {
    #[default]
    NoneByDefault,
    AllByDefault,
    OnlyLast,
}

/// State for environments that number equations.
#[derive(Debug, Default)]
pub(super) struct NumberedEnvState<'arena> {
    pub(super) mode: NumberingMode,
    pub(super) suppress_next_number: bool,
    pub(super) custom_next_tag: Option<EquationTag<'arena>>,
    pub(super) label: Option<&'arena str>,
    pub(super) num_rows: Option<NonZeroU16>,
}

impl<'arena> NumberedEnvState<'arena> {
    pub(super) fn next_equation_tag(
        &mut self,
        equation_counter: &mut usize,
        is_last: bool,
        arena: &'arena Arena,
    ) -> Result<Option<EquationTag<'arena>>, ()> {
        if matches!(self.mode, NumberingMode::OnlyLast) && !is_last {
            // Not the last row; do nothing for now.
            return Ok(None);
        }
        // A custom number takes precedence over suppression.
        if let Some(custom_tag) = self.custom_next_tag.take() {
            // The state has already been cleared here through `take()`.
            Ok(Some(custom_tag))
        } else if self.suppress_next_number || matches!(self.mode, NumberingMode::NoneByDefault) {
            // Clear the flag.
            self.suppress_next_number = false;
            Ok(None)
        } else {
            *equation_counter = equation_counter.checked_add(1).ok_or(())?;
            // FIXME: Use the `int_format_into` feature once it's stabilized.
            let mut buffer = String::new();
            write!(buffer, "{}", equation_counter).map_err(|_| ())?;
            let tag = arena.alloc_str(&buffer);
            Ok(Some(EquationTag {
                text: tag,
                parenthesized: true,
            }))
        }
    }
}
