use std::borrow::Cow;
use std::fmt::{self, Write};
use std::num::NonZeroU16;

use percent_encoding::{AsciiSet, CONTROLS, percent_encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use rustc_hash::FxHashMap;

use crate::fmt::new_line_and_indent;
use crate::itoa::append_u8_as_hex;
use crate::length::{Length, LengthUnit, LengthValue};
use crate::symbol::MathMLOperator;
use crate::table::{
    Alignment, ArraySpec, BORDER_TOP_DASHED, BORDER_TOP_SOLID, ColumnGenerator, LineType,
    RIGHT_ALIGN, RowLabelInfo,
};
use crate::{
    attribute::{
        FracAttr, HtmlTextSize, HtmlTextStyle, LetterAttr, MathSpacing, Notation, OpAttrs, Size,
        Style,
    },
    super_char::SuperChar,
};

/// A wrapper around `str` to do HTML escaping
/// in the `Display` impl.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
struct EscapeHtml<'a>(&'a str);

impl fmt::Display for EscapeHtml<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '<' => write!(f, "&lt;")?,
                '>' => write!(f, "&gt;")?,
                '&' => write!(f, "&amp;")?,
                '"' => write!(f, "&quot;")?,
                '\'' => write!(f, "&#39;")?,
                _ => write!(f, "{c}")?,
            }
        }
        Ok(())
    }
}

/// According to: https://stackoverflow.com/a/26119120
const FRAGMENT_SAFE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// Stores the contents of [`Node::AHref`].
/// Needs to be a separate `struct` to keep [`Node`]
/// 4 words in size
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AHref<'arena> {
    pub href: &'arena str,
    pub text: &'arena str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RowAttrs {
    // `color: …;` CSS property
    pub color: Option<(u8, u8, u8)>,
    // `style` attribute
    pub style: Option<Style>,
    // `math-shift: compact;` CSS property
    pub math_shift_compact: bool,
}

impl RowAttrs {
    pub const DEFAULT: Self = Self {
        color: None,
        style: None,
        math_shift_compact: false,
    };
}

/// A single sub/sup pair in [`Multicripts`].
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MultiscriptPair<'arena> {
    pub sub: &'arena Node<'arena>,
    pub sup: &'arena Node<'arena>,
}

/// AST node
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Node<'arena> {
    /// `<mn>...</mn>`
    Number(&'arena str),
    /// `<mi>...</mi>` for a [`SuperChar`].
    IdentifierChar(SuperChar, LetterAttr),
    /// `<mi>...</mi>` for a string.
    IdentifierStr(&'arena str),
    /// `<mo>...</mo>` for a single character.
    Operator {
        op: MathMLOperator,
        attrs: OpAttrs,
        size: Option<Size>,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    /// `<mo>...</mo>` for a string.
    PseudoOp {
        attrs: OpAttrs,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
        name: &'arena str,
    },
    /// `<mspace width="..."/>`
    Space(Length),
    /// `<msub>...</msub>`
    Sub {
        target: &'arena Node<'arena>,
        symbol: &'arena Node<'arena>,
    },
    /// `<msup>...</msup>`
    Sup {
        target: &'arena Node<'arena>,
        symbol: &'arena Node<'arena>,
    },
    /// `<msubsup>...</msubsup>`
    SubSup {
        target: &'arena Node<'arena>,
        sub: &'arena Node<'arena>,
        sup: &'arena Node<'arena>,
    },
    /// `<mover accent="true">...</mover>`
    OverAccent(MathMLOperator, OpAttrs, &'arena Node<'arena>),
    /// `<munder accentunder="true">...</munder>`
    UnderAccent(MathMLOperator, OpAttrs, &'arena Node<'arena>),
    /// `<mover>...</mover>`
    Over {
        symbol: &'arena Node<'arena>,
        target: &'arena Node<'arena>,
    },
    /// `<munder>...</munder>`
    Under {
        symbol: &'arena Node<'arena>,
        target: &'arena Node<'arena>,
    },
    /// `<munderover>...</munderover>`
    UnderOver {
        target: &'arena Node<'arena>,
        under: &'arena Node<'arena>,
        over: &'arena Node<'arena>,
    },
    /// `<msqrt>...</msqrt>`
    Sqrt(&'arena Node<'arena>),
    /// `<mroot>...</mroot>`
    Root(&'arena Node<'arena>, &'arena Node<'arena>),
    /// `<mfrac>...</mfrac>`
    Frac {
        /// Numerator
        num: &'arena Node<'arena>,
        /// Denominator
        denom: &'arena Node<'arena>,
        /// Line thickness
        lt_value: LengthValue,
        lt_unit: LengthUnit,
        attr: Option<FracAttr>,
    },
    /// `<mrow>...</mrow>`
    Row {
        nodes: &'arena [&'arena Node<'arena>],
        attrs: RowAttrs,
    },
    /// `<mpadded>...</mpadded>`
    Padded {
        node: &'arena Node<'arena>,
        width_0: bool,
        height_0: bool,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    /// `<mphantom>...</mphantom>`
    Phantom { node: &'arena Node<'arena> },
    /// `<mtext>...</mtext>`.
    /// The `str` gets HTML-escaped.
    Text(Option<HtmlTextStyle>, Option<HtmlTextSize>, &'arena str),
    /// `<mtext><a href="...">...</a></mtext>`.
    /// The link and text get HTML-escaped.
    AHref(&'arena AHref<'arena>),
    /// `<mtext><a href="...">...</a></mtext>`.
    /// The link and text get HTML-escaped.
    EqRef(&'arena str),
    /// `<mtable>...</mtable>` for matrices and similar constructs
    Table {
        align: Alignment,
        style: Option<Style>,
        content: &'arena [&'arena Node<'arena>],
    },
    /// `<mtable>...</mtable>` for equation arrays like the `align` environment
    EquationArray {
        align: Alignment,
        last_row_info: Option<&'arena RowLabelInfo<'arena>>,
        content: &'arena [&'arena Node<'arena>],
    },
    /// `<mtable>...</mtable>` for the `multline` environment
    MultLine {
        num_rows: NonZeroU16,
        last_row_info: Option<&'arena RowLabelInfo<'arena>>,
        content: &'arena [&'arena Node<'arena>],
    },
    /// `<mtable>...</mtable>` for arrays
    Array {
        style: Option<Style>,
        array_spec: &'arena ArraySpec<'arena>,
        content: &'arena [&'arena Node<'arena>],
    },
    /// `<mtd>...</mtd>`
    ColumnSeparator,
    /// `<mtr>...</mtr>`
    RowSeparator {
        label_info: Option<&'arena RowLabelInfo<'arena>>,
        border_top: Option<LineType>,
    },
    /// `<menclose>...</menclose>`
    Enclose {
        content: &'arena Node<'arena>,
        notation: Notation,
    },
    /// `<mmultiscripts>...</mmultiscripts>`
    /// Double pointer indirection is to keep `Node`'s size down.
    /// Ideally we would use some sort of thinslice type
    Multiscripts {
        base: &'arena Node<'arena>,
        pre: &'arena &'arena [MultiscriptPair<'arena>],
        post: &'arena &'arena [MultiscriptPair<'arena>],
    },
    /// This node is used for displaying unknown commands.
    /// It's `<merror>` with a custom CSS class
    /// to override the default ugly yellow background
    UnknownCommand(&'arena str),
}

#[cfg(target_arch = "wasm32")]
static_assertions::assert_eq_size!(Node<'_>, [usize; 4]);

macro_rules! writeln_indent {
    ($buf:expr, $indent:expr, $($tail:tt)+) => {
        new_line_and_indent($buf, $indent);
        write!($buf, $($tail)+)?
    };
}

impl Node<'_> {
    pub const EMPTY_ROW: Self = Self::Row {
        nodes: &[],
        attrs: RowAttrs::DEFAULT,
    };
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default, rename_all = "kebab-case"))]
pub struct CssClassNames {
    /// The CSS class name to use for unknown commands, if `ignore_unknown_commands` is `true`.
    pub unknown_command: Cow<'static, str>,
}

impl Default for CssClassNames {
    fn default() -> Self {
        Self {
            unknown_command: "math-core-unknown-cmd".into(),
        }
    }
}

#[derive(Debug)]
pub struct Emitter<'state> {
    s: String,
    label_map: &'state FxHashMap<Box<str>, Box<str>>,
    css_classes: &'state CssClassNames,
}

impl<'state> Emitter<'state> {
    pub fn new(
        s: String,
        label_map: &'state FxHashMap<Box<str>, Box<str>>,
        css_classes: &'state CssClassNames,
    ) -> Self {
        Self {
            s,
            label_map,
            css_classes,
        }
    }

    pub fn emit(&mut self, node: &Node<'_>, base_indent: usize) -> std::fmt::Result {
        // Compute the indentation for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        // Get the base indentation out of the way.
        new_line_and_indent(&mut self.s, base_indent);

        match *node {
            Node::Number(number) => {
                write!(self.s, "<mn>{number}</mn>")?;
            }
            Node::IdentifierChar(letter, attr) => {
                let is_upright = matches!(attr, LetterAttr::ForcedUpright);
                // Only set "mathvariant" if we are not transforming the letter.
                let write_mrow;
                if is_upright {
                    write!(self.s, "<mrow><mspace/><mi mathvariant=\"normal\">")?;
                    write_mrow = true;
                } else if letter.try_as_char().is_none() {
                    // check if multi-char
                    write!(self.s, "<mrow><mspace/><mi>")?;
                    write_mrow = true;
                } else {
                    write!(self.s, "<mi>")?;
                    write_mrow = false;
                }
                write!(self.s, "{letter}</mi>")?;
                if write_mrow {
                    write!(self.s, "</mrow>")?;
                }
            }
            Node::Operator {
                op,
                attrs,
                left,
                right,
                size,
            } => {
                emit_operator_attributes(&mut self.s, attrs, left, right)?;
                if let Some(size) = size {
                    write!(
                        self.s,
                        " minsize=\"{}\" maxsize=\"{}\"",
                        <&str>::from(size),
                        <&str>::from(size),
                    )?;
                }
                write!(self.s, ">{op}</mo>")?;
            }
            Node::PseudoOp {
                attrs,
                left,
                right,
                name,
            } => {
                emit_operator_attributes(&mut self.s, attrs, left, right)?;
                write!(self.s, ">{name}</mo>")?;
            }
            Node::IdentifierStr(letters) => {
                // The "<mrow>" with "<mspace/>" is needed to prevent Firefox from adding
                // extra space around multi-letter identifiers.
                debug_assert!(
                    letters.chars().count() > 1,
                    "single-letter IdentifierStr should be IdentifierChar"
                );
                write!(
                    self.s,
                    "<mrow><mspace/><mi>{}</mi></mrow>",
                    EscapeHtml(letters)
                )?;
            }
            Node::Text(text_style, text_size, letters) => {
                write!(self.s, "<mtext")?;
                if let Some(size) = text_size {
                    write!(self.s, " style=\"font-size:{}\"", <&str>::from(size))?;
                }
                let (open, close) = match text_style {
                    None => ("", ""),
                    Some(HtmlTextStyle::Bold) => ("<b>", "</b>"),
                    Some(HtmlTextStyle::Italic) => ("<i>", "</i>"),
                    Some(HtmlTextStyle::BoldItalic) => ("<b><i>", "</i></b>"),
                    Some(HtmlTextStyle::Emphasis) => ("<em>", "</em>"),
                    Some(HtmlTextStyle::Typewriter) => ("<code>", "</code>"),
                    Some(HtmlTextStyle::SmallCaps) => {
                        ("<span style=\"font-variant-caps: small-caps\">", "</span>")
                    }
                    Some(HtmlTextStyle::SansSerif) => {
                        ("<span class=\"math-core-sans-serif-font\">", "</span>")
                    }
                    Some(HtmlTextStyle::Serif) => {
                        ("<span class=\"math-core-serif-font\">", "</span>")
                    }
                    Some(HtmlTextStyle::Strikethrough) => ("<s>", "</s>"),
                    Some(HtmlTextStyle::Underline) => ("<u>", "</u>"),
                };
                write!(self.s, ">{open}{}{close}</mtext>", EscapeHtml(letters))?;
            }
            Node::Space(space) => {
                write!(self.s, "<mspace ")?;

                if space.is_negative() {
                    write!(self.s, "style=\"margin-left:")?;
                    space.push_to_string(&mut self.s);
                    write!(self.s, ";")?;
                } else {
                    write!(self.s, "width=\"")?;
                    space.push_to_string(&mut self.s);
                    // Work-around for a Firefox bug that causes "rem" to not be processed correctly
                    if matches!(space.unit, LengthUnit::Rem) {
                        write!(self.s, "\" style=\"width:")?;
                        space.push_to_string(&mut self.s);
                    }
                }

                write!(self.s, "\"/>")?;
            }
            // The following nodes have exactly two children.
            ref node @ (Node::Sub {
                symbol: second,
                target: first,
            }
            | Node::Sup {
                symbol: second,
                target: first,
            }
            | Node::Over {
                symbol: second,
                target: first,
            }
            | Node::Under {
                symbol: second,
                target: first,
            }
            | Node::Root(second, first)) => {
                let (open, close) = match node {
                    Node::Sub { .. } => ("<msub>", "</msub>"),
                    Node::Sup { .. } => ("<msup>", "</msup>"),
                    Node::Over { .. } => ("<mover>", "</mover>"),
                    Node::Under { .. } => ("<munder>", "</munder>"),
                    Node::Root(_, _) => ("<mroot>", "</mroot>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(self.s, "{open}")?;
                self.emit(first, child_indent)?;
                self.emit(second, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "{close}");
            }
            // The following nodes have exactly three children.
            ref node @ (Node::SubSup {
                target: first,
                sub: second,
                sup: third,
            }
            | Node::UnderOver {
                target: first,
                under: second,
                over: third,
            }) => {
                let (open, close) = match node {
                    Node::SubSup { .. } => ("<msubsup>", "</msubsup>"),
                    Node::UnderOver { .. } => ("<munderover>", "</munderover>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(self.s, "{open}")?;
                self.emit(first, child_indent)?;
                self.emit(second, child_indent)?;
                self.emit(third, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "{close}");
            }
            Node::Multiscripts { base, pre, post } => {
                write!(self.s, "<mmultiscripts>")?;
                self.emit(base, child_indent)?;
                for &MultiscriptPair { sub, sup } in *post {
                    self.emit(sub, child_indent)?;
                    self.emit(sup, child_indent)?;
                }
                if !pre.is_empty() {
                    writeln_indent!(&mut self.s, child_indent, "<mprescripts/>");
                    for &MultiscriptPair { sub, sup } in *pre {
                        self.emit(sub, child_indent)?;
                        self.emit(sup, child_indent)?;
                    }
                }

                writeln_indent!(&mut self.s, base_indent, "</mmultiscripts>");
            }
            ref
            node @ (Node::OverAccent(op, attr, target) | Node::UnderAccent(op, attr, target)) => {
                let (open, close) = match node {
                    Node::OverAccent(_, _, _) => ("<mover accent=\"true\">", "</mover>"),
                    Node::UnderAccent(_, _, _) => ("<munder accentunder=\"true\">", "</munder>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(self.s, "{open}")?;
                self.emit(target, child_indent)?;
                writeln_indent!(&mut self.s, child_indent, "<mo");
                attr.write_to(&mut self.s);
                write!(self.s, ">{op}</mo>")?;
                writeln_indent!(&mut self.s, base_indent, "{close}");
            }
            Node::Sqrt(content) => {
                write!(self.s, "<msqrt>")?;
                self.emit(content, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "</msqrt>");
            }
            Node::Frac {
                num,
                denom: den,
                lt_value: line_length,
                lt_unit: line_unit,
                attr,
            } => {
                write!(self.s, "<mfrac")?;
                let lt = Length::from_parts(line_length, line_unit);
                if let Some(lt) = lt {
                    write!(self.s, " linethickness=\"")?;
                    lt.push_to_string(&mut self.s);
                    write!(self.s, "\"")?;
                }
                if let Some(style) = attr {
                    write!(self.s, "{}", <&str>::from(style))?;
                }
                write!(self.s, ">")?;
                self.emit(num, child_indent)?;
                self.emit(den, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "</mfrac>");
            }
            Node::Row {
                nodes,
                attrs:
                    RowAttrs {
                        color,
                        style,
                        math_shift_compact,
                    },
            } => {
                write!(self.s, "<mrow")?;

                if color.is_some() || math_shift_compact {
                    write!(self.s, " style=\"")?;
                    if let Some((r, g, b)) = color {
                        write!(self.s, "color:#")?;
                        append_u8_as_hex(&mut self.s, r);
                        append_u8_as_hex(&mut self.s, g);
                        append_u8_as_hex(&mut self.s, b);
                        write!(self.s, ";")?;
                    }
                    if math_shift_compact {
                        write!(self.s, "math-shift:compact;")?;
                    }
                    write!(self.s, "\"")?;
                }

                if let Some(style) = style {
                    write!(self.s, "{}", <&str>::from(style))?;
                }

                write!(self.s, ">")?;

                if nodes.is_empty() {
                    write!(self.s, "</mrow>")?;
                } else {
                    for node in nodes {
                        self.emit(node, child_indent)?;
                    }
                    writeln_indent!(&mut self.s, base_indent, "</mrow>");
                }
            }
            Node::Padded {
                node,
                width_0,
                height_0,
                left,
                right,
            } => {
                write!(self.s, "<mpadded")?;
                if width_0 {
                    write!(self.s, r#" width="0""#)?;
                }
                if height_0 {
                    write!(self.s, r#" height="0""#)?;
                }
                if left.is_some() || right.is_some() {
                    write!(self.s, " style=\"")?;
                    if let Some(left) = left {
                        write!(self.s, "padding-left:{};", <&str>::from(left))?;
                    }
                    if let Some(right) = right {
                        write!(self.s, "padding-right:{};", <&str>::from(right))?;
                    }
                    write!(self.s, "\"")?;
                }
                write!(self.s, ">")?;
                self.emit(node, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "</mpadded>");
            }
            Node::Phantom { node } => {
                write!(self.s, "<mphantom>")?;
                self.emit(node, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "</mphantom>");
            }
            Node::Table {
                content,
                align,
                style,
            } => {
                let mtd_opening = ColumnGenerator::new_predefined(align);

                write!(self.s, "<mtable")?;
                if let Some(style) = style {
                    write!(self.s, "{}", <&str>::from(style))?;
                }
                write!(self.s, ">")?;
                self.emit_table(base_indent, child_indent, content, mtd_opening, None, None)?;
            }
            ref node @ (Node::EquationArray {
                last_row_info,
                content,
                ..
            }
            | Node::MultLine {
                last_row_info,
                content,
                ..
            }) => {
                let (mtd_opening, numbering_cols) = match node {
                    Node::EquationArray { align, .. } => {
                        (ColumnGenerator::new_predefined(*align), NumberColums::Wide)
                    }
                    Node::MultLine { num_rows, .. } => (
                        ColumnGenerator::new_multline(*num_rows),
                        NumberColums::Narrow,
                    ),
                    _ => unreachable!(),
                };

                write!(
                    &mut self.s,
                    r#"<mtable displaystyle="true" scriptlevel="0" style="width: 100%">"#
                )?;
                self.emit_table(
                    base_indent,
                    child_indent,
                    content,
                    mtd_opening,
                    Some(numbering_cols),
                    last_row_info,
                )?;
            }
            Node::Array {
                style,
                content,
                array_spec,
            } => {
                let mtd_opening = ColumnGenerator::new_custom(array_spec);
                write!(self.s, "<mtable")?;
                // `border_left` (from a leading `|`/`:`) and `border_top` (from a leading
                // `\hline`/`\hdashline`) both go into a single `style` attribute on the table.
                if array_spec.border_left.is_some() || array_spec.border_top.is_some() {
                    write!(self.s, " style=\"")?;
                    match array_spec.border_left {
                        Some(LineType::Solid) => {
                            write!(self.s, "border-left: 0.05em solid currentcolor;")?;
                        }
                        Some(LineType::Dashed) => {
                            write!(self.s, "border-left: 0.05em dashed currentcolor;")?;
                        }
                        None => (),
                    }
                    match array_spec.border_top {
                        Some(LineType::Solid) => write!(self.s, "{BORDER_TOP_SOLID}")?,
                        Some(LineType::Dashed) => write!(self.s, "{BORDER_TOP_DASHED}")?,
                        None => (),
                    }
                    write!(self.s, "\"")?;
                }
                if let Some(style) = style {
                    write!(self.s, "{}", <&str>::from(style))?;
                }
                write!(self.s, ">")?;
                self.emit_table(base_indent, child_indent, content, mtd_opening, None, None)?;
            }
            Node::RowSeparator { .. } | Node::ColumnSeparator => {
                // This should only appear in tables where it is handled in `emit_table`.
                if cfg!(debug_assertions) {
                    panic!("ColumnSeparator node should be handled in emit_table");
                }
            }
            Node::Enclose { content, notation } => {
                write!(self.s, "<menclose notation=\"")?;
                let mut first = true;
                if notation.contains(Notation::UP_DIAGONAL) {
                    write!(self.s, "updiagonalstrike")?;
                    first = false;
                }
                if notation.contains(Notation::DOWN_DIAGONAL) {
                    if !first {
                        write!(self.s, " ")?;
                    }
                    write!(self.s, "downdiagonalstrike")?;
                }
                if notation.contains(Notation::HORIZONTAL) {
                    if !first {
                        write!(self.s, " ")?;
                    }
                    write!(self.s, "horizontalstrike")?;
                }
                write!(self.s, "\">")?;
                self.emit(content, child_indent)?;
                if notation.contains(Notation::UP_DIAGONAL) {
                    writeln_indent!(
                        &mut self.s,
                        child_indent,
                        "<mrow class=\"menclose-updiagonalstrike\"></mrow>"
                    );
                }
                if notation.contains(Notation::DOWN_DIAGONAL) {
                    writeln_indent!(
                        &mut self.s,
                        child_indent,
                        "<mrow class=\"menclose-downdiagonalstrike\"></mrow>"
                    );
                }
                if notation.contains(Notation::HORIZONTAL) {
                    writeln_indent!(
                        &mut self.s,
                        child_indent,
                        "<mrow class=\"menclose-horizontalstrike\"></mrow>"
                    );
                }
                writeln_indent!(&mut self.s, base_indent, "</menclose>");
            }
            Node::AHref(&AHref { href, text }) => {
                write!(
                    self.s,
                    r#"<mtext><a href="{}">{}</a></mtext>"#,
                    EscapeHtml(href),
                    EscapeHtml(text)
                )?;
            }
            Node::EqRef(label) => {
                let tag: &str = match self.label_map.get(label) {
                    Some(tag) => tag,
                    None => "??",
                };
                write!(
                    self.s,
                    r##"<mtext><a href="#{}">({})</a></mtext>"##,
                    percent_encode(label.as_bytes(), FRAGMENT_SAFE),
                    EscapeHtml(tag)
                )?;
            }
            Node::UnknownCommand(cmd_name) => {
                write!(
                    self.s,
                    r#"<merror class="{}"><mtext>\{cmd_name}</mtext></merror>"#,
                    self.css_classes.unknown_command
                )?;
            }
        }
        Ok(())
    }

    fn emit_table(
        &mut self,
        base_indent: usize,
        child_indent: usize,
        content: &[&Node<'_>],
        mut col_gen: ColumnGenerator,
        numbering_cols: Option<NumberColums>,
        last_row_info: Option<&RowLabelInfo>,
    ) -> Result<(), std::fmt::Error> {
        let child_indent2 = if base_indent > 0 {
            child_indent.saturating_add(1)
        } else {
            0
        };
        let child_indent3 = if base_indent > 0 {
            child_indent2.saturating_add(1)
        } else {
            0
        };
        writeln_indent!(&mut self.s, child_indent, "<mtr>");
        if let Some(numbering_cols) = numbering_cols {
            numbering_cols.initial_dummy_column(&mut self.s, child_indent2)?;
        }
        col_gen.write_next_mtd(&mut self.s, child_indent2)?;
        for node in content {
            match **node {
                Node::ColumnSeparator => {
                    writeln_indent!(&mut self.s, child_indent2, "</mtd>");
                    col_gen.write_next_mtd(&mut self.s, child_indent2)?;
                }
                Node::RowSeparator {
                    label_info,
                    border_top,
                } => {
                    writeln_indent!(&mut self.s, child_indent2, "</mtd>");
                    if let Some(numbering_cols) = numbering_cols {
                        write_equation_num(
                            &mut self.s,
                            child_indent2,
                            child_indent3,
                            label_info,
                            numbering_cols,
                        )?;
                    }
                    writeln_indent!(&mut self.s, child_indent, "</mtr>");
                    writeln_indent!(&mut self.s, child_indent, "<mtr>");
                    if let Some(numbering_cols) = numbering_cols {
                        numbering_cols.initial_dummy_column(&mut self.s, child_indent2)?;
                    }
                    col_gen.reset_to_new_row();
                    // A `\hline`/`\hdashline` right after this `\\` becomes the top border of the
                    // row we're about to open. MathML `<mtr>` borders aren't rendered by all
                    // browsers (notably Firefox), so it's applied per-cell by the column generator.
                    col_gen.set_row_border_top(border_top);
                    col_gen.write_next_mtd(&mut self.s, child_indent2)?;
                }
                _ => {
                    self.emit(node, child_indent3)?;
                }
            }
        }
        writeln_indent!(&mut self.s, child_indent2, "</mtd>");
        if let Some(numbering_cols) = numbering_cols {
            write_equation_num(
                &mut self.s,
                child_indent2,
                child_indent3,
                last_row_info,
                numbering_cols,
            )?;
        }
        writeln_indent!(&mut self.s, child_indent, "</mtr>");
        writeln_indent!(&mut self.s, base_indent, "</mtable>");
        Ok(())
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.s
    }
}

fn emit_operator_attributes(
    s: &mut String,
    attrs: OpAttrs,
    left: Option<MathSpacing>,
    right: Option<MathSpacing>,
) -> std::fmt::Result {
    s.push_str("<mo");
    attrs.write_to(s);
    match (left, right) {
        (Some(left), Some(right)) => {
            write!(
                s,
                " lspace=\"{}\" rspace=\"{}\"",
                <&str>::from(left),
                <&str>::from(right)
            )?;
        }
        (Some(left), None) => {
            write!(s, " lspace=\"{}\"", <&str>::from(left))?;
        }
        (None, Some(right)) => {
            write!(s, " rspace=\"{}\"", <&str>::from(right))?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberColums {
    Narrow,
    Wide,
}

impl NumberColums {
    fn dummy_column_opening(
        self,
        s: &mut String,
        child_indent2: usize,
    ) -> Result<(), std::fmt::Error> {
        match self {
            NumberColums::Narrow => {
                writeln_indent!(s, child_indent2, r#"<mtd style="width: 7.5%"#);
            }
            NumberColums::Wide => {
                writeln_indent!(s, child_indent2, r#"<mtd style="width: 50%"#);
            }
        }
        Ok(())
    }

    /// Initial dummy column for equation numbering for keeping alignment.
    #[inline]
    fn initial_dummy_column(
        self,
        s: &mut String,
        child_indent2: usize,
    ) -> Result<(), std::fmt::Error> {
        self.dummy_column_opening(s, child_indent2)?;
        write!(s, "\"></mtd>")?;
        Ok(())
    }
}

fn write_equation_num(
    s: &mut String,
    child_indent2: usize,
    child_indent3: usize,
    label_info: Option<&RowLabelInfo>,
    numbering_cols: NumberColums,
) -> Result<(), std::fmt::Error> {
    numbering_cols.dummy_column_opening(s, child_indent2)?;
    if let Some(label_info) = label_info {
        write!(s, r#";{RIGHT_ALIGN}""#)?;
        if let Some(link_target) = label_info.link_target {
            write!(
                s,
                r#" id="{}">"#,
                percent_encode(link_target.as_bytes(), FRAGMENT_SAFE)
            )?;
        } else {
            write!(s, ">")?;
        }
        writeln_indent!(
            s,
            child_indent3,
            "<mtext>({})</mtext>",
            EscapeHtml(label_info.tag)
        );
        writeln_indent!(s, child_indent2, "</mtd>");
    } else {
        write!(s, "\"></mtd>")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::symbol;
    use super::super::table::{ColumnAlignment, ColumnSpecEntry};
    use super::*;

    const WORD: usize = std::mem::size_of::<usize>();

    #[test]
    fn test_struct_sizes() {
        assert!(std::mem::size_of::<Node>() <= 4 * WORD, "size of Node");
    }

    pub fn render<'a, 'b>(node: &'a Node<'b>) -> String
    where
        'a: 'b,
    {
        let output = String::new();
        let label_map = FxHashMap::default();
        let css_classes = CssClassNames::default();
        let mut emitter = Emitter::new(output, &label_map, &css_classes);
        emitter.emit(node, 0).unwrap();
        emitter.into_string()
    }

    #[test]
    fn render_number() {
        assert_eq!(render(&Node::Number("3.14")), "<mn>3.14</mn>");
    }

    #[test]
    fn render_single_letter_ident() {
        assert_eq!(
            render(&Node::IdentifierChar('x'.into(), LetterAttr::Default)),
            "<mi>x</mi>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('Γ'.into(), LetterAttr::ForcedUpright)),
            "<mrow><mspace/><mi mathvariant=\"normal\">Γ</mi></mrow>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝑥'.into(), LetterAttr::Default)),
            "<mi>𝑥</mi>"
        );
    }

    #[test]
    fn render_operator_with_spacing() {
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.as_op(),
                attrs: OpAttrs::empty(),
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::FourMu),
                size: None,
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0.2222em\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.as_op(),
                attrs: OpAttrs::empty(),
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::Zero),
                size: None,
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::IDENTICAL_TO.as_op(),
                attrs: OpAttrs::empty(),
                left: Some(MathSpacing::Zero),
                right: None,
                size: None,
            }),
            "<mo lspace=\"0\">≡</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::PLUS_SIGN.as_op(),
                attrs: OpAttrs::FORM_PREFIX,
                left: None,
                right: None,
                size: None,
            }),
            "<mo form=\"prefix\">+</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::N_ARY_SUMMATION.as_op(),
                attrs: OpAttrs::NO_MOVABLE_LIMITS,
                left: None,
                right: None,
                size: None,
            }),
            "<mo movablelimits=\"false\">∑</mo>"
        );
    }

    #[test]
    fn render_pseudo_operator() {
        assert_eq!(
            render(&Node::PseudoOp {
                attrs: OpAttrs::empty(),
                left: Some(MathSpacing::ThreeMu),
                right: Some(MathSpacing::ThreeMu),
                name: "sin"
            }),
            "<mo lspace=\"0.1667em\" rspace=\"0.1667em\">sin</mo>"
        );
    }

    #[test]
    fn render_collected_letters() {
        assert_eq!(
            render(&Node::IdentifierStr("sin")),
            "<mrow><mspace/><mi>sin</mi></mrow>"
        );
    }

    #[test]
    fn render_space() {
        assert_eq!(
            render(&Node::Space(Length::new(1.0, LengthUnit::Em))),
            "<mspace width=\"1em\"/>"
        );
    }

    #[test]
    fn render_subscript() {
        assert_eq!(
            render(&Node::Sub {
                target: &Node::IdentifierChar('x'.into(), LetterAttr::Default),
                symbol: &Node::Number("2"),
            }),
            "<msub><mi>x</mi><mn>2</mn></msub>"
        );
    }

    #[test]
    fn render_superscript() {
        assert_eq!(
            render(&Node::Sup {
                target: &Node::IdentifierChar('x'.into(), LetterAttr::Default),
                symbol: &Node::Number("2"),
            }),
            "<msup><mi>x</mi><mn>2</mn></msup>"
        );
    }

    #[test]
    fn render_sub_sup() {
        assert_eq!(
            render(&Node::SubSup {
                target: &Node::IdentifierChar('x'.into(), LetterAttr::Default),
                sub: &Node::Number("1"),
                sup: &Node::Number("2"),
            }),
            "<msubsup><mi>x</mi><mn>1</mn><mn>2</mn></msubsup>"
        );
    }

    #[test]
    fn render_over_op() {
        assert_eq!(
            render(&Node::OverAccent(
                symbol::MACRON.as_op(),
                OpAttrs::STRETCHY_FALSE,
                &Node::IdentifierChar('x'.into(), LetterAttr::Default),
            )),
            "<mover accent=\"true\"><mi>x</mi><mo stretchy=\"false\">¯</mo></mover>"
        );
        assert_eq!(
            render(&Node::OverAccent(
                symbol::OVERLINE.as_op(),
                OpAttrs::empty(),
                &Node::IdentifierChar('x'.into(), LetterAttr::Default),
            )),
            "<mover accent=\"true\"><mi>x</mi><mo>‾</mo></mover>"
        );
    }

    #[test]
    fn render_under_op() {
        assert_eq!(
            render(&Node::UnderAccent(
                symbol::COMBINING_LOW_LINE.as_op(),
                OpAttrs::empty(),
                &Node::IdentifierChar('x'.into(), LetterAttr::Default),
            )),
            "<munder accentunder=\"true\"><mi>x</mi><mo>\u{332}</mo></munder>"
        );
    }

    #[test]
    fn render_overset() {
        assert_eq!(
            render(&Node::Over {
                symbol: &Node::Operator {
                    op: symbol::EXCLAMATION_MARK.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                    size: None,
                },
                target: &Node::Operator {
                    op: symbol::EQUALS_SIGN.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                    size: None,
                },
            }),
            "<mover><mo>=</mo><mo>!</mo></mover>"
        );
    }

    #[test]
    fn render_underset() {
        assert_eq!(
            render(&Node::Under {
                symbol: &Node::IdentifierChar('θ'.into(), LetterAttr::Default),
                target: &Node::PseudoOp {
                    attrs: OpAttrs::FORCE_MOVABLE_LIMITS,
                    left: Some(MathSpacing::ThreeMu),
                    right: Some(MathSpacing::ThreeMu),
                    name: "min",
                },
            }),
            "<munder><mo movablelimits=\"true\" lspace=\"0.1667em\" rspace=\"0.1667em\">min</mo><mi>θ</mi></munder>"
        );
    }

    #[test]
    fn render_under_over() {
        assert_eq!(
            render(&Node::UnderOver {
                target: &Node::IdentifierChar('x'.into(), LetterAttr::Default),
                under: &Node::Number("1"),
                over: &Node::Number("2"),
            }),
            "<munderover><mi>x</mi><mn>1</mn><mn>2</mn></munderover>"
        );
    }

    #[test]
    fn render_sqrt() {
        assert_eq!(
            render(&Node::Sqrt(&Node::IdentifierChar(
                'x'.into(),
                LetterAttr::Default
            ))),
            "<msqrt><mi>x</mi></msqrt>"
        );
    }

    #[test]
    fn render_root() {
        assert_eq!(
            render(&Node::Root(
                &Node::Number("3"),
                &Node::IdentifierChar('x'.into(), LetterAttr::Default),
            )),
            "<mroot><mi>x</mi><mn>3</mn></mroot>"
        );
    }

    #[test]
    fn render_frac() {
        let num = &Node::Number("1");
        let denom = &Node::Number("2");
        let (lt_value, lt_unit) = Length::none().into_parts();
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: None,
            }),
            "<mfrac><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: Some(FracAttr::DisplayStyleTrue),
            }),
            "<mfrac displaystyle=\"true\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: Some(FracAttr::DisplayStyleFalse),
            }),
            "<mfrac displaystyle=\"false\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        let (lt_value, lt_unit) = Length::new(-1.0, LengthUnit::Rem).into_parts();
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: None,
            }),
            "<mfrac linethickness=\"-1rem\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value: LengthValue(1.0),
                lt_unit: LengthUnit::Em,
                attr: None,
            }),
            "<mfrac linethickness=\"1em\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value: LengthValue(-1.0),
                lt_unit: LengthUnit::Ex,
                attr: None,
            }),
            "<mfrac linethickness=\"-1ex\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        let (lt_value, lt_unit) = Length::new(2.0, LengthUnit::Rem).into_parts();
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: None,
            }),
            "<mfrac linethickness=\"2rem\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        let (lt_value, lt_unit) = Length::zero().into_parts();
        assert_eq!(
            render(&Node::Frac {
                num,
                denom,
                lt_value,
                lt_unit,
                attr: Some(FracAttr::DisplayStyleTrue),
            }),
            "<mfrac linethickness=\"0\" displaystyle=\"true\"><mn>1</mn><mn>2</mn></mfrac>"
        );
    }

    #[test]
    fn render_row() {
        let nodes = &[
            &Node::IdentifierChar('x'.into(), LetterAttr::Default),
            &Node::Operator {
                op: symbol::EQUALS_SIGN.as_op(),
                attrs: OpAttrs::empty(),
                left: None,
                right: None,
                size: None,
            },
            &Node::Number("1"),
        ];

        assert_eq!(
            render(&Node::Row {
                nodes,
                attrs: RowAttrs {
                    style: Some(Style::Display),
                    ..RowAttrs::DEFAULT
                }
            }),
            "<mrow displaystyle=\"true\" scriptlevel=\"0\"><mi>x</mi><mo>=</mo><mn>1</mn></mrow>"
        );

        assert_eq!(
            render(&Node::Row {
                nodes,
                attrs: RowAttrs {
                    color: Some((0, 0, 0)),
                    ..RowAttrs::DEFAULT
                }
            }),
            "<mrow style=\"color:#000000;\"><mi>x</mi><mo>=</mo><mn>1</mn></mrow>"
        );
    }

    #[test]
    fn render_padded() {
        assert_eq!(
            render(&Node::Padded {
                node: &Node::Number("x"),
                width_0: true,
                height_0: false,
                left: None,
                right: Some(MathSpacing::FourMu),
            }),
            "<mpadded width=\"0\" style=\"padding-right:0.2222em;\"><mn>x</mn></mpadded>"
        );
    }

    #[test]
    fn render_phantom() {
        assert_eq!(
            render(&Node::Phantom {
                node: &Node::Number("x")
            }),
            "<mphantom><mn>x</mn></mphantom>"
        );
    }

    #[test]
    fn render_sized_operator() {
        assert_eq!(
            render(&Node::Operator {
                op: symbol::LEFT_PARENTHESIS.as_op(),
                attrs: OpAttrs::empty(),
                size: Some(Size::Scale1),
                left: None,
                right: None,
            }),
            "<mo minsize=\"1.2em\" maxsize=\"1.2em\">(</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::SOLIDUS.as_op(),
                attrs: OpAttrs::STRETCHY_TRUE | OpAttrs::SYMMETRIC_TRUE,
                size: Some(Size::Scale3),
                left: Some(MathSpacing::Zero),
                right: Some(MathSpacing::Zero),
            }),
            "<mo stretchy=\"true\" symmetric=\"true\" lspace=\"0\" rspace=\"0\" minsize=\"2.047em\" maxsize=\"2.047em\">/</mo>"
        );
    }

    #[test]
    fn render_text() {
        assert_eq!(
            render(&Node::Text(None, None, "hello")),
            "<mtext>hello</mtext>"
        );
    }

    #[test]
    fn render_table() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator {
                label_info: None,
                border_top: None,
            },
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::Table {
                content: &nodes,
                align: Alignment::Centered,
                style: None,
            }),
            "<mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_equation_array() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator {
                label_info: Some(&RowLabelInfo {
                    tag: "1",
                    link_target: None,
                }),
                border_top: None,
            },
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        let info_with_tag = RowLabelInfo {
            tag: "2",
            link_target: None,
        };
        assert_eq!(
            render(&Node::EquationArray {
                content: &nodes,
                align: Alignment::Centered,
                last_row_info: Some(&info_with_tag),
            }),
            "<mtable displaystyle=\"true\" scriptlevel=\"0\" style=\"width: 100%\"><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd><mtd style=\"width: 50%;text-align: right;justify-items: end;\"><mtext>(1)</mtext></mtd></mtr><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd><mtd style=\"width: 50%;text-align: right;justify-items: end;\"><mtext>(2)</mtext></mtd></mtr></mtable>"
        );

        assert_eq!(
            render(&Node::EquationArray {
                content: &nodes,
                align: Alignment::Centered,
                last_row_info: None,
            }),
            "<mtable displaystyle=\"true\" scriptlevel=\"0\" style=\"width: 100%\"><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd><mtd style=\"width: 50%;text-align: right;justify-items: end;\"><mtext>(1)</mtext></mtd></mtr><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd><mtd style=\"width: 50%\"></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_array() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator {
                label_info: None,
                border_top: None,
            },
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::Array {
                style: None,
                content: &nodes,
                array_spec: &ArraySpec {
                    border_left: None,
                    border_top: None,
                    is_sub: false,
                    column_spec: &[
                        ColumnSpecEntry::WithContent {
                            alignment: ColumnAlignment::LeftJustified,
                            border_right: None
                        },
                        ColumnSpecEntry::WithContent {
                            alignment: ColumnAlignment::Centered,
                            border_right: None
                        },
                    ],
                },
            }),
            "<mtable><mtr><mtd style=\"text-align: left;justify-items: start;\"><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd style=\"text-align: left;justify-items: start;\"><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_array_with_hlines() {
        // A leading `\hline` (solid) sets `border_top` on the array spec, and an `\hline`
        // after the `\\` sets `border_top` on the corresponding `RowSeparator`.
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator {
                label_info: None,
                border_top: Some(LineType::Dashed),
            },
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::Array {
                style: None,
                content: &nodes,
                array_spec: &ArraySpec {
                    border_left: None,
                    border_top: Some(LineType::Solid),
                    is_sub: false,
                    column_spec: &[
                        ColumnSpecEntry::WithContent {
                            alignment: ColumnAlignment::Centered,
                            border_right: None
                        },
                        ColumnSpecEntry::WithContent {
                            alignment: ColumnAlignment::Centered,
                            border_right: None
                        },
                    ],
                },
            }),
            "<mtable style=\"border-top: 0.05em solid currentcolor;\"><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd style=\"border-top: 0.05em dashed currentcolor;\"><mn>3</mn></mtd><mtd style=\"border-top: 0.05em dashed currentcolor;\"><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_multiscript() {
        assert_eq!(
            render(&Node::Multiscripts {
                base: &Node::IdentifierChar('x'.into(), LetterAttr::Default),
                pre: &const {
                    &[MultiscriptPair {
                        sub: &Node::Number("1"),
                        sup: &Node::EMPTY_ROW,
                    }]
                },
                post: &const { &[] },
            }),
            "<mmultiscripts><mi>x</mi><mprescripts/><mn>1</mn><mrow></mrow></mmultiscripts>"
        );
    }

    #[test]
    fn render_text_transform() {
        assert_eq!(
            render(&Node::IdentifierChar('a'.into(), LetterAttr::ForcedUpright)),
            "<mrow><mspace/><mi mathvariant=\"normal\">a</mi></mrow>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('a'.into(), LetterAttr::ForcedUpright)),
            "<mrow><mspace/><mi mathvariant=\"normal\">a</mi></mrow>"
        );
        assert_eq!(
            render(&Node::IdentifierStr("abc")),
            "<mrow><mspace/><mi>abc</mi></mrow>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝐚'.into(), LetterAttr::Default)),
            "<mi>𝐚</mi>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝒂'.into(), LetterAttr::Default)),
            "<mi>𝒂</mi>"
        );
        assert_eq!(
            render(&Node::IdentifierStr("𝒂𝒃𝒄")),
            "<mrow><mspace/><mi>𝒂𝒃𝒄</mi></mrow>"
        );
    }

    #[test]
    fn render_enclose() {
        let content = Node::Row {
            nodes: &[
                &Node::IdentifierChar('a'.into(), LetterAttr::Default),
                &Node::IdentifierChar('b'.into(), LetterAttr::Default),
                &Node::IdentifierChar('c'.into(), LetterAttr::Default),
            ],
            attrs: RowAttrs::DEFAULT,
        };

        assert_eq!(
            render(&Node::Enclose {
                content: &content,
                notation: Notation::UP_DIAGONAL | Notation::DOWN_DIAGONAL
            }),
            "<menclose notation=\"updiagonalstrike downdiagonalstrike\"><mrow><mi>a</mi><mi>b</mi><mi>c</mi></mrow><mrow class=\"menclose-updiagonalstrike\"></mrow><mrow class=\"menclose-downdiagonalstrike\"></mrow></menclose>"
        );
    }

    #[test]
    fn fragment_encoding() {
        let ascii_samples = b"\x00\x01\x1F !\"#$%&'()*+,-./09:;<=>?@AZ[\\]^_`az{|}~\x7F";
        let encoded = percent_encode(ascii_samples, FRAGMENT_SAFE).to_string();
        assert_eq!(
            encoded,
            "%00%01%1F%20!%22%23$%25&'()*+,-./09:;%3C=%3E?@AZ%5B%5C%5D%5E_%60az%7B%7C%7D~%7F"
        );
    }
}
