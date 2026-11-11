use alloc::string::String;
use core::{fmt::Write, mem};

use mathml_renderer::{
    arena::Arena,
    ast::Node,
    attribute::Style,
    symbol,
    table::{Alignment, ArraySpec, EquationTag, RowLabelInfo, TableKind, TableRow},
};

use crate::{TaggingStrategy, character_class::StretchableOp};

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

    /// `true` for environments whose columns come in pairs of a right-aligned and a left-aligned
    /// column (the `align` family).
    #[inline]
    fn has_alternating_columns(self) -> bool {
        matches!(self, Env::Align | Env::AlignStar | Env::Aligned)
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

    /// `true` for environments in which a `\\` right before `\end{...}` doesn't start a new row.
    ///
    /// This is the case for all environments that are used inside a formula (those based on
    /// LaTeX's `array`). The display environments of amsmath (`align`, `gather`, `multline`)
    /// start a new, empty row instead, which in numbered environments even gets its own equation
    /// number.
    #[inline]
    fn ignores_trailing_newline(self) -> bool {
        matches!(
            self,
            Env::Array
                | Env::DArray
                | Env::Subarray
                | Env::Aligned
                | Env::Gathered
                | Env::Matrix
                | Env::PMatrix
                | Env::BMatrix
                | Env::Bmatrix
                | Env::VMatrix
                | Env::Vmatrix
                | Env::Cases
                | Env::RCases
                | Env::DCases
                | Env::DRCases
        )
    }

    /// `true` for the display environments of amsmath (`align`, `equation`, `gather`, `multline`
    /// and their starred forms).
    ///
    /// Such an environment is a whole equation rather than a part of one: it spans the full
    /// width of the line and carries the equation numbers. It therefore has to be the only
    /// content of a block equation, and has no place in inline math, where `aligned` and
    /// `gathered` are the alternatives.
    #[inline]
    pub(super) fn is_display_env(self) -> bool {
        matches!(
            self,
            Env::Align
                | Env::AlignStar
                | Env::Equation
                | Env::EquationStar
                | Env::Gather
                | Env::GatherStar
                | Env::MultLine
                | Env::MultLineStar
        )
    }

    /// How the rows of a display environment are numbered.
    #[inline]
    pub(super) fn numbering(self, tags: TaggingStrategy) -> EquationNumbering<'static> {
        // The starred forms are not numbered, and with `TaggingStrategy::None`, nothing is.
        let automatic = matches!(
            self,
            Env::Align | Env::Equation | Env::Gather | Env::MultLine
        ) && !matches!(tags, TaggingStrategy::None);
        // `multline` is a single equation which is spread over several rows.
        let single_number = matches!(self, Env::MultLine | Env::MultLineStar);
        EquationNumbering::new(automatic, single_number)
    }

    pub(super) fn new_state(self) -> EnvState {
        EnvState {
            allow_columns: self.allows_columns(),
            alternating_columns: self.has_alternating_columns(),
            nested: false,
            meaningful_newlines: !matches!(self, Env::Equation | Env::EquationStar),
            allow_hlines: self.allows_hlines(),
            allow_shove: self.allows_shove(),
            ignore_trailing_newline: self.ignores_trailing_newline(),
            forbid_tag: matches!(self, Env::Aligned | Env::Gathered),
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
        rows: &'arena [TableRow<'arena>],
        array_spec: Option<&'arena ArraySpec<'arena>>,
    ) -> Node<'arena> {
        match self {
            Env::Align | Env::AlignStar => Node::Table {
                kind: TableKind::EquationArray(Alignment::Alternating),
                rows,
            },
            Env::Aligned => Node::Table {
                kind: TableKind::Inline {
                    align: Alignment::Alternating,
                    style: Some(Style::Display),
                },
                rows,
            },
            Env::Equation | Env::EquationStar | Env::Gather | Env::GatherStar => Node::Table {
                kind: TableKind::EquationArray(Alignment::Centered),
                rows,
            },
            Env::Gathered => Node::Table {
                kind: TableKind::Inline {
                    align: Alignment::Centered,
                    style: Some(Style::Display),
                },
                rows,
            },
            Env::Matrix => Node::Table {
                kind: TableKind::Inline {
                    align: Alignment::Centered,
                    style: Some(Style::Text),
                },
                rows,
            },
            Env::MultLine | Env::MultLineStar => Node::Table {
                kind: TableKind::MultLine,
                rows,
            },
            Env::Cases | Env::RCases => Node::Table {
                kind: TableKind::Inline {
                    align: Alignment::Cases,
                    style: Some(Style::Text),
                },
                rows,
            },
            Env::DCases | Env::DRCases => Node::Table {
                kind: TableKind::Inline {
                    align: Alignment::Cases,
                    style: Some(Style::Display),
                },
                rows,
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
                    rows,
                    array_spec,
                }
            }
            Env::PMatrix | Env::BMatrix | Env::Bmatrix | Env::VMatrix | Env::Vmatrix => {
                Node::Table {
                    kind: TableKind::Inline {
                        align: Alignment::Centered,
                        style: Some(Style::Text),
                    },
                    rows,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EnvState {
    /// `true` if we are inside an environment that allows columns (`&`).
    pub allow_columns: bool,
    /// `true` if the columns of the environment come in pairs of a right-aligned and a
    /// left-aligned column (as in `align` environments).
    pub alternating_columns: bool,
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
    /// `true` if a `\\` right before the end of the environment doesn't start a new row.
    pub ignore_trailing_newline: bool,
    /// `true` if we are inside `aligned` or `gathered`, however deeply. These environments
    /// cut their content off from the numbering of the equation: `\tag` is not allowed in
    /// them, and `\notag` has no effect.
    pub forbid_tag: bool,
}

/// The numbering of the equation which is being parsed.
///
/// A block equation has exactly one of these, because a display environment is always the
/// whole equation. What is numbered are the rows of that environment, or, if there is none,
/// the equation as a whole, which then counts as a single row.
#[derive(Debug)]
pub(super) struct EquationNumbering<'arena> {
    /// `true` if a row gets a number without one having been set with `\tag`.
    automatic: bool,
    /// `true` if all rows share one number, which is shown in the last row.
    single_number: bool,
    pub(super) suppress_next_number: bool,
    pub(super) custom_next_tag: Option<EquationTag<'arena>>,
    pub(super) label: Option<&'arena str>,
}

impl<'arena> EquationNumbering<'arena> {
    pub(super) fn new(automatic: bool, single_number: bool) -> Self {
        EquationNumbering {
            automatic,
            single_number,
            suppress_next_number: false,
            custom_next_tag: None,
            label: None,
        }
    }

    /// Determine the number and the link target of the row which has just ended, and get
    /// ready for the next row.
    pub(super) fn finish_row(
        &mut self,
        equation_counter: &mut usize,
        is_last: bool,
        arena: &'arena Arena,
    ) -> Result<Option<RowLabelInfo<'arena>>, ()> {
        if self.single_number && !is_last {
            // The number is shown in the last row, so everything is kept until then.
            return Ok(None);
        }
        let suppressed = mem::take(&mut self.suppress_next_number);
        // A row without a number cannot be referred to, so its label is dropped.
        let link_target = self.label.take();
        // A custom number takes precedence over suppression.
        let tag = if let Some(custom_tag) = self.custom_next_tag.take() {
            custom_tag
        } else if suppressed || !self.automatic {
            return Ok(None);
        } else {
            *equation_counter = equation_counter.checked_add(1).ok_or(())?;
            // FIXME: Use the `int_format_into` feature once it's stabilized.
            let mut buffer = String::new();
            write!(buffer, "{}", equation_counter).map_err(|_| ())?;
            EquationTag {
                text: arena.alloc_str(&buffer),
                parenthesized: true,
            }
        };
        Ok(Some(RowLabelInfo { tag, link_target }))
    }
}
