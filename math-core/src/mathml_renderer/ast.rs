use std::fmt::Write;
use std::num::NonZeroU16;

#[cfg(feature = "serde")]
use serde::Serialize;

use super::attribute::{
    FracAttr, HtmlTextStyle, LetterAttr, MathSpacing, Notation, OpAttr, RowAttr, Size, StretchMode,
    Style,
};
use super::fmt::new_line_and_indent;
use super::itoa::append_u8_as_hex;
use super::length::{Length, LengthUnit, LengthValue};
use super::symbol::{MathMLOperator, StretchableOp, Stretchy};
use super::table::{Alignment, ArraySpec, ColumnGenerator, LineType, RIGHT_ALIGN};

/// AST node
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Node<'arena> {
    /// `<mn>...</mn>`
    Number(&'arena str),
    /// `<mi>...</mi>` for a single character.
    IdentifierChar(char, LetterAttr),
    StretchableOp(StretchableOp, StretchMode),
    /// `<mo>...</mo>` for a single character.
    Operator {
        op: MathMLOperator,
        attr: Option<OpAttr>,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    /// `<mo>...</mo>` for a string.
    PseudoOp {
        attr: Option<OpAttr>,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
        name: &'arena str,
    },
    /// `<mi>...</mi>` for a string.
    IdentifierStr(bool, &'arena str),
    /// `<mspace width="..."/>`
    Space(Length),
    /// `<msub>...</msub>`
    Subscript {
        target: &'arena Node<'arena>,
        symbol: &'arena Node<'arena>,
    },
    /// `<msup>...</msup>`
    Superscript {
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
    OverOp(MathMLOperator, Option<OpAttr>, &'arena Node<'arena>),
    /// `<munder accent="true">...</munder>`
    UnderOp(MathMLOperator, &'arena Node<'arena>),
    /// `<mover>...</mover>`
    Overset {
        symbol: &'arena Node<'arena>,
        target: &'arena Node<'arena>,
    },
    /// `<munder>...</munder>`
    Underset {
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
        attr: RowAttr,
    },
    Fenced {
        style: Option<Style>,
        open: Option<StretchableOp>,
        close: Option<StretchableOp>,
        content: &'arena Node<'arena>,
    },
    SizedParen(Size, StretchableOp),
    /// `<mtext>...</mtext>`
    Text(Option<HtmlTextStyle>, &'arena str),
    /// `<mtable>...</mtable>` with predefined alignment
    Table {
        content: &'arena [&'arena Node<'arena>],
        align: Alignment,
        style: Option<Style>,
    },
    /// `<mtable>...</mtable>` with predefined alignment
    EquationArray {
        content: &'arena [&'arena Node<'arena>],
        align: Alignment,
        last_equation_num: Option<NonZeroU16>,
    },
    /// `<mtable>...</mtable>` with custom alignment
    Array {
        style: Option<Style>,
        content: &'arena [&'arena Node<'arena>],
        array_spec: &'arena ArraySpec<'arena>,
    },
    /// `<mtd>...</mtd>`
    ColumnSeparator,
    /// `<mtr>...</mtr>`
    RowSeparator(Option<NonZeroU16>),
    /// <menclose>...</menclose>
    Enclose {
        content: &'arena Node<'arena>,
        notation: Notation,
    },
    Slashed(&'arena Node<'arena>),
    Multiscript {
        base: &'arena Node<'arena>,
        sub: Option<&'arena Node<'arena>>,
        sup: Option<&'arena Node<'arena>>,
    },
    HardcodedMathML(&'static str),
    /// This node is used when the parser needs to return a node,
    /// but does not want to emit anything.
    Dummy,
}

macro_rules! writeln_indent {
    ($buf:expr, $indent:expr, $($tail:tt)+) => {
        new_line_and_indent($buf, $indent);
        write!($buf, $($tail)+)?
    };
}

impl Node<'_> {
    pub fn emit(&self, s: &mut String, base_indent: usize) -> std::fmt::Result {
        // Compute the indent for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        // Get the base indent out of the way, as long as we are not in a "pseudo" node.
        if !matches!(self, Node::Dummy | Node::RowSeparator(_)) {
            new_line_and_indent(s, base_indent);
        }

        match self {
            Node::Number(number) => {
                write!(s, "<mn>{number}</mn>")?;
            }
            Node::IdentifierChar(letter, attr) => {
                let is_upright = matches!(attr, LetterAttr::ForcedUpright);
                // Only set "mathvariant" if we are not transforming the letter.
                if is_upright {
                    write!(s, "<mpadded><mi mathvariant=\"normal\">")?;
                } else {
                    write!(s, "<mi>")?;
                }
                let c = *letter;
                write!(s, "{c}</mi>")?;
                if is_upright {
                    write!(s, "</mpadded>")?;
                }
            }
            Node::StretchableOp(op, stretch_mode) => {
                if op.ordinary_spacing() && matches!(stretch_mode, StretchMode::NoStretch) {
                    write!(s, "<mi>{}</mi>", char::from(*op))?;
                } else {
                    emit_stretchy_op(s, *stretch_mode, Some(*op))?;
                }
            }
            Node::Operator {
                op,
                attr,
                left,
                right,
            } => {
                emit_operator_attributes(s, *attr, *left, *right)?;
                write!(s, ">{}</mo>", char::from(op))?;
            }
            Node::PseudoOp {
                attr,
                left,
                right,
                name: text,
            } => {
                emit_operator_attributes(s, *attr, *left, *right)?;
                write!(s, ">{text}</mo>")?;
            }
            node @ (Node::IdentifierStr(_, letters) | Node::Text(_, letters)) => {
                let (open, close) = match node {
                    Node::IdentifierStr(with_tf, _) => {
                        // This is only needed to prevent Firefox from adding extra space around
                        // multi-letter ASCII identifiers.
                        if *with_tf {
                            ("<mi>", "</mi>")
                        } else {
                            ("<mpadded><mi>", "</mi></mpadded>")
                        }
                    }
                    Node::Text(text_style, _) => match text_style {
                        None => ("<mtext>", "</mtext>"),
                        Some(HtmlTextStyle::Bold) => ("<mtext><b>", "</b></mtext>"),
                        Some(HtmlTextStyle::Italic) => ("<mtext><i>", "</i></mtext>"),
                        Some(HtmlTextStyle::Emphasis) => ("<mtext><em>", "</em></mtext>"),
                        Some(HtmlTextStyle::Typewriter) => ("<mtext><code>", "</code></mtext>"),
                        Some(HtmlTextStyle::SmallCaps) => (
                            "<mtext><span style=\"font-variant-caps: small-caps\">",
                            "</span></mtext>",
                        ),
                    },
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(s, "{open}{letters}{close}")?;
            }
            Node::Space(space) => {
                write!(s, "<mspace width=\"")?;
                space.push_to_string(s);
                // Work-around for a Firefox bug that causes "rem" to not be processed correctly
                if matches!(space.unit, LengthUnit::Rem) {
                    write!(s, "\" style=\"width:")?;
                    space.push_to_string(s);
                }
                write!(s, "\"/>")?;
            }
            // The following nodes have exactly two children.
            node @ (Node::Subscript {
                symbol: second,
                target: first,
            }
            | Node::Superscript {
                symbol: second,
                target: first,
            }
            | Node::Overset {
                symbol: second,
                target: first,
            }
            | Node::Underset {
                symbol: second,
                target: first,
            }
            | Node::Root(second, first)) => {
                let (open, close) = match node {
                    Node::Subscript { .. } => ("<msub>", "</msub>"),
                    Node::Superscript { .. } => ("<msup>", "</msup>"),
                    Node::Overset { .. } => ("<mover>", "</mover>"),
                    Node::Underset { .. } => ("<munder>", "</munder>"),
                    Node::Root(_, _) => ("<mroot>", "</mroot>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(s, "{open}")?;
                first.emit(s, child_indent)?;
                second.emit(s, child_indent)?;
                writeln_indent!(s, base_indent, "{close}");
            }
            // The following nodes have exactly three children.
            node @ (Node::SubSup {
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
                write!(s, "{open}")?;
                first.emit(s, child_indent)?;
                second.emit(s, child_indent)?;
                third.emit(s, child_indent)?;
                writeln_indent!(s, base_indent, "{close}");
            }
            Node::Multiscript { base, sub, sup } => {
                write!(s, "<mmultiscripts>")?;
                base.emit(s, child_indent)?;
                writeln_indent!(s, child_indent, "<mprescripts/>");
                if let Some(sub) = sub {
                    sub.emit(s, child_indent)?;
                } else {
                    writeln_indent!(s, child_indent, "<mrow></mrow>");
                }
                if let Some(sup) = sup {
                    sup.emit(s, child_indent)?;
                } else {
                    writeln_indent!(s, child_indent, "<mrow></mrow>");
                }
                writeln_indent!(s, base_indent, "</mmultiscripts>");
            }
            Node::OverOp(op, attr, target) => {
                write!(s, "<mover accent=\"true\">")?;
                target.emit(s, child_indent)?;
                writeln_indent!(s, child_indent, "<mo");
                if let Some(attr) = attr {
                    write!(s, "{}", <&str>::from(attr))?;
                }
                write!(s, ">{}</mo>", char::from(op))?;
                writeln_indent!(s, base_indent, "</mover>");
            }
            Node::UnderOp(op, target) => {
                write!(s, "<munder accentunder=\"true\">")?;
                target.emit(s, child_indent)?;
                writeln_indent!(s, child_indent, "<mo>{}</mo>", char::from(op));
                writeln_indent!(s, base_indent, "</munder>");
            }
            Node::Sqrt(content) => {
                write!(s, "<msqrt>")?;
                content.emit(s, child_indent)?;
                writeln_indent!(s, base_indent, "</msqrt>");
            }
            Node::Frac {
                num,
                denom: den,
                lt_value: line_length,
                lt_unit: line_unit,
                attr,
            } => {
                write!(s, "<mfrac")?;
                let lt = Length::from_parts(*line_length, *line_unit);
                if let Some(lt) = lt {
                    write!(s, " linethickness=\"")?;
                    lt.push_to_string(s);
                    write!(s, "\"")?;
                }
                if let Some(style) = attr {
                    write!(s, "{}", <&str>::from(style))?;
                }
                write!(s, ">")?;
                num.emit(s, child_indent)?;
                den.emit(s, child_indent)?;
                writeln_indent!(s, base_indent, "</mfrac>");
            }
            Node::Row { nodes, attr: style } => {
                match style {
                    RowAttr::None => {
                        write!(s, "<mrow>")?;
                    }
                    RowAttr::Style(style) => {
                        write!(s, "<mrow{}>", <&str>::from(style))?;
                    }
                    RowAttr::Color(r, g, b) => {
                        write!(s, "<mrow style=\"color:#")?;
                        append_u8_as_hex(s, *r);
                        append_u8_as_hex(s, *g);
                        append_u8_as_hex(s, *b);
                        write!(s, ";\">")?;
                    }
                }
                for node in nodes.iter() {
                    node.emit(s, child_indent)?;
                }
                writeln_indent!(s, base_indent, "</mrow>");
            }
            Node::Fenced {
                open,
                close,
                content,
                style,
            } => {
                match style {
                    Some(style) => write!(s, "<mrow{}>", <&str>::from(style))?,
                    None => write!(s, "<mrow>")?,
                };
                new_line_and_indent(s, child_indent);
                emit_stretchy_op(s, StretchMode::Fence, *open)?;
                content.emit(s, child_indent)?;
                new_line_and_indent(s, child_indent);
                emit_stretchy_op(s, StretchMode::Fence, *close)?;
                writeln_indent!(s, base_indent, "</mrow>");
            }
            Node::SizedParen(size, paren) => {
                write!(
                    s,
                    "<mo maxsize=\"{}\" minsize=\"{}\"",
                    <&str>::from(size),
                    <&str>::from(size)
                )?;
                match paren.stretchy {
                    Stretchy::PrePostfix | Stretchy::Never => {
                        write!(s, " stretchy=\"true\" symmetric=\"true\"")?;
                    }
                    Stretchy::AlwaysAsymmetric => {
                        write!(s, " symmetric=\"true\"")?;
                    }
                    _ => {}
                }
                write!(s, ">{}</mo>", char::from(*paren))?;
            }
            Node::Slashed(node) => match node {
                Node::IdentifierChar(x, attr) => {
                    if matches!(attr, LetterAttr::ForcedUpright) {
                        write!(s, "<mi mathvariant=\"normal\">{x}&#x0338;</mi>")?;
                    } else {
                        write!(s, "<mi>{x}&#x0338;</mi>")?;
                    }
                }
                Node::Operator { op, .. } => {
                    write!(s, "<mo>{}&#x0338;</mo>", char::from(op))?;
                }
                n => n.emit(s, base_indent)?,
            },
            Node::Table {
                content,
                align,
                style,
            } => {
                let mtd_opening = ColumnGenerator::new_predefined(*align);

                write!(s, "<mtable")?;
                if let Some(style) = style {
                    write!(s, "{}", <&str>::from(style))?;
                }
                write!(s, ">")?;
                emit_table(
                    s,
                    base_indent,
                    child_indent,
                    content,
                    mtd_opening,
                    false,
                    None,
                )?;
            }
            Node::EquationArray {
                content,
                align,
                last_equation_num,
            } => {
                let mtd_opening = ColumnGenerator::new_predefined(*align);

                write!(
                    s,
                    r#"<mtable displaystyle="true" scriptlevel="0" style="width: 100%">"#
                )?;
                emit_table(
                    s,
                    base_indent,
                    child_indent,
                    content,
                    mtd_opening,
                    true,
                    last_equation_num.as_ref().copied(),
                )?;
            }
            Node::Array {
                style,
                content,
                array_spec,
            } => {
                let mtd_opening = ColumnGenerator::new_custom(array_spec);
                write!(s, "<mtable")?;
                match array_spec.beginning_line {
                    Some(LineType::Solid) => {
                        write!(s, " style=\"border-left: 0.05em solid currentcolor\"")?;
                    }
                    Some(LineType::Dashed) => {
                        write!(s, " style=\"border-left: 0.05em dashed currentcolor\"")?;
                    }
                    _ => (),
                }
                if let Some(style) = style {
                    write!(s, "{}", <&str>::from(style))?;
                }
                write!(s, ">")?;
                emit_table(
                    s,
                    base_indent,
                    child_indent,
                    content,
                    mtd_opening,
                    false,
                    None,
                )?;
            }
            Node::ColumnSeparator => {
                // This should only appear in tables where it is handled in `emit_table`.
                if cfg!(debug_assertions) {
                    panic!("ColumnSeparator node should be handled in emit_table");
                }
            }
            Node::RowSeparator(_) => {
                // This should only appear in tables where it is handled in `emit_table`.
                // However, we are currently not yet properly ensuring this fact,
                // so we just ignore it here.
            }
            Node::Enclose { content, notation } => {
                let notation = *notation;
                write!(s, "<menclose notation=\"")?;
                let mut first = true;
                if notation.contains(Notation::UP_DIAGONAL) {
                    write!(s, "updiagonalstrike")?;
                    first = false;
                }
                if notation.contains(Notation::DOWN_DIAGONAL) {
                    if !first {
                        write!(s, " ")?;
                    }
                    write!(s, "downdiagonalstrike")?;
                }
                if notation.contains(Notation::HORIZONTAL) {
                    if !first {
                        write!(s, " ")?;
                    }
                    write!(s, "horizontalstrike")?;
                }
                write!(s, "\">")?;
                content.emit(s, child_indent)?;
                if notation.contains(Notation::UP_DIAGONAL) {
                    writeln_indent!(
                        s,
                        child_indent,
                        "<mrow class=\"menclose-updiagonalstrike\"></mrow>"
                    );
                }
                if notation.contains(Notation::DOWN_DIAGONAL) {
                    writeln_indent!(
                        s,
                        child_indent,
                        "<mrow class=\"menclose-downdiagonalstrike\"></mrow>"
                    );
                }
                if notation.contains(Notation::HORIZONTAL) {
                    writeln_indent!(
                        s,
                        child_indent,
                        "<mrow class=\"menclose-horizontalstrike\"></mrow>"
                    );
                }
                writeln_indent!(s, base_indent, "</menclose>");
            }
            Node::HardcodedMathML(mathml) => {
                write!(s, "{mathml}")?;
            }
            Node::Dummy => {
                // Do nothing.
            }
        };
        Ok(())
    }
}

fn emit_operator_attributes(
    s: &mut String,
    attr: Option<OpAttr>,
    left: Option<MathSpacing>,
    right: Option<MathSpacing>,
) -> std::fmt::Result {
    match attr {
        Some(attributes) => write!(s, "<mo{}", <&str>::from(attributes))?,
        None => write!(s, "<mo")?,
    };
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
    };
    Ok(())
}

fn emit_table(
    s: &mut String,
    base_indent: usize,
    child_indent: usize,
    content: &[&Node<'_>],
    mut col_gen: ColumnGenerator,
    is_equation_array: bool,
    last_equation_num: Option<NonZeroU16>,
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
    col_gen.reset_columns();
    writeln_indent!(s, child_indent, "<mtr>");
    if is_equation_array {
        // Add a dummy column to help keep everything centered.
        writeln_indent!(s, child_indent2, r#"<mtd style="width: 50%"></mtd>"#);
    }
    col_gen.write_next_mtd(s, child_indent2)?;
    for node in content.iter() {
        match node {
            Node::ColumnSeparator => {
                writeln_indent!(s, child_indent2, "</mtd>");
                col_gen.write_next_mtd(s, child_indent2)?;
            }
            Node::RowSeparator(equation_counter) => {
                writeln_indent!(s, child_indent2, "</mtd>");
                if is_equation_array {
                    write_equation_num(
                        s,
                        child_indent2,
                        child_indent3,
                        equation_counter.as_ref().copied(),
                    )?;
                }
                writeln_indent!(s, child_indent, "</mtr>");
                writeln_indent!(s, child_indent, "<mtr>");
                col_gen.reset_columns();
                if is_equation_array {
                    // Add a dummy column to help keep everything centered.
                    writeln_indent!(s, child_indent2, r#"<mtd style="width: 50%"></mtd>"#);
                }
                col_gen.write_next_mtd(s, child_indent2)?;
            }
            node => {
                node.emit(s, child_indent3)?;
            }
        }
    }
    writeln_indent!(s, child_indent2, "</mtd>");
    if is_equation_array {
        write_equation_num(s, child_indent2, child_indent3, last_equation_num)?;
    }
    writeln_indent!(s, child_indent, "</mtr>");
    writeln_indent!(s, base_indent, "</mtable>");
    Ok(())
}

fn write_equation_num(
    s: &mut String,
    child_indent2: usize,
    child_indent3: usize,
    equation_counter: Option<NonZeroU16>,
) -> Result<(), std::fmt::Error> {
    writeln_indent!(s, child_indent2, r#"<mtd style="width: 50%"#);
    if let Some(equation_counter) = equation_counter {
        write!(s, r#";{}">"#, RIGHT_ALIGN)?;
        writeln_indent!(s, child_indent3, "<mtext>({})</mtext>", equation_counter);
        writeln_indent!(s, child_indent2, "</mtd>");
    } else {
        write!(s, "\"></mtd>")?;
    }
    Ok(())
}

fn emit_stretchy_op(
    s: &mut String,
    stretch_mode: StretchMode,
    op: Option<StretchableOp>,
) -> std::fmt::Result {
    if let Some(op) = op {
        match (stretch_mode, op.stretchy) {
            (StretchMode::Fence, Stretchy::Never)
            | (StretchMode::Middle, Stretchy::PrePostfix | Stretchy::Never) => {
                write!(s, "<mo stretchy=\"true\">")?;
            }
            (
                StretchMode::NoStretch,
                Stretchy::Always | Stretchy::PrePostfix | Stretchy::AlwaysAsymmetric,
            ) => {
                write!(s, "<mo stretchy=\"false\">")?;
            }

            (StretchMode::Middle, Stretchy::AlwaysAsymmetric) => {
                write!(s, "<mo symmetric=\"true\">")?;
            }
            _ => {
                write!(s, "<mo>")?;
            }
        }
        write!(s, "{}", char::from(op))?;
    } else {
        write!(s, "<mo>")?;
    }
    write!(s, "</mo>")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::symbol;
    use super::super::table::{ColumnAlignment, ColumnSpec};
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
        let mut output = String::new();
        node.emit(&mut output, 0).unwrap();
        output
    }

    #[test]
    fn render_number() {
        assert_eq!(render(&Node::Number("3.14")), "<mn>3.14</mn>");
    }

    #[test]
    fn render_single_letter_ident() {
        assert_eq!(
            render(&Node::IdentifierChar('x', LetterAttr::Default)),
            "<mi>x</mi>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('Γ', LetterAttr::ForcedUpright)),
            "<mpadded><mi mathvariant=\"normal\">Γ</mi></mpadded>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝑥', LetterAttr::Default)),
            "<mi>𝑥</mi>"
        );
    }

    #[test]
    fn render_operator_with_spacing() {
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.as_op(),
                attr: None,
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::FourMu),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0.2222em\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.as_op(),
                attr: None,
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::Zero),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::IDENTICAL_TO.as_op(),
                attr: None,
                left: Some(MathSpacing::Zero),
                right: None,
            }),
            "<mo lspace=\"0\">≡</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::PLUS_SIGN.as_op(),
                attr: Some(OpAttr::FormPrefix),
                left: None,
                right: None,
            }),
            "<mo form=\"prefix\">+</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::N_ARY_SUMMATION.as_op(),
                attr: Some(OpAttr::NoMovableLimits),
                left: None,
                right: None,
            }),
            "<mo movablelimits=\"false\">∑</mo>"
        );
    }

    #[test]
    fn render_pseudo_operator() {
        assert_eq!(
            render(&Node::PseudoOp {
                attr: None,
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
            render(&Node::IdentifierStr(false, "sin")),
            "<mpadded><mi>sin</mi></mpadded>"
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
            render(&Node::Subscript {
                target: &Node::IdentifierChar('x', LetterAttr::Default),
                symbol: &Node::Number("2"),
            }),
            "<msub><mi>x</mi><mn>2</mn></msub>"
        );
    }

    #[test]
    fn render_superscript() {
        assert_eq!(
            render(&Node::Superscript {
                target: &Node::IdentifierChar('x', LetterAttr::Default),
                symbol: &Node::Number("2"),
            }),
            "<msup><mi>x</mi><mn>2</mn></msup>"
        );
    }

    #[test]
    fn render_sub_sup() {
        assert_eq!(
            render(&Node::SubSup {
                target: &Node::IdentifierChar('x', LetterAttr::Default),
                sub: &Node::Number("1"),
                sup: &Node::Number("2"),
            }),
            "<msubsup><mi>x</mi><mn>1</mn><mn>2</mn></msubsup>"
        );
    }

    #[test]
    fn render_over_op() {
        assert_eq!(
            render(&Node::OverOp(
                symbol::MACRON.as_op(),
                Some(OpAttr::StretchyFalse),
                &Node::IdentifierChar('x', LetterAttr::Default),
            )),
            "<mover accent=\"true\"><mi>x</mi><mo stretchy=\"false\">¯</mo></mover>"
        );
        assert_eq!(
            render(&Node::OverOp(
                symbol::OVERLINE.as_op(),
                None,
                &Node::IdentifierChar('x', LetterAttr::Default),
            )),
            "<mover accent=\"true\"><mi>x</mi><mo>‾</mo></mover>"
        );
    }

    #[test]
    fn render_under_op() {
        assert_eq!(
            render(&Node::UnderOp(
                symbol::LOW_LINE.as_op(),
                &Node::IdentifierChar('x', LetterAttr::Default),
            )),
            "<munder accentunder=\"true\"><mi>x</mi><mo>_</mo></munder>"
        );
    }

    #[test]
    fn render_overset() {
        assert_eq!(
            render(&Node::Overset {
                symbol: &Node::Operator {
                    op: symbol::EXCLAMATION_MARK,
                    attr: None,
                    left: None,
                    right: None
                },
                target: &Node::Operator {
                    op: symbol::EQUALS_SIGN.as_op(),
                    attr: None,
                    left: None,
                    right: None
                },
            }),
            "<mover><mo>=</mo><mo>!</mo></mover>"
        );
    }

    #[test]
    fn render_underset() {
        assert_eq!(
            render(&Node::Underset {
                symbol: &Node::IdentifierChar('θ', LetterAttr::Default),
                target: &Node::PseudoOp {
                    attr: Some(OpAttr::ForceMovableLimits),
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
                target: &Node::IdentifierChar('x', LetterAttr::Default),
                under: &Node::Number("1"),
                over: &Node::Number("2"),
            }),
            "<munderover><mi>x</mi><mn>1</mn><mn>2</mn></munderover>"
        );
    }

    #[test]
    fn render_sqrt() {
        assert_eq!(
            render(&Node::Sqrt(&Node::IdentifierChar('x', LetterAttr::Default))),
            "<msqrt><mi>x</mi></msqrt>"
        );
    }

    #[test]
    fn render_root() {
        assert_eq!(
            render(&Node::Root(
                &Node::Number("3"),
                &Node::IdentifierChar('x', LetterAttr::Default),
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
            &Node::IdentifierChar('x', LetterAttr::Default),
            &Node::Operator {
                op: symbol::EQUALS_SIGN.as_op(),
                attr: None,
                left: None,
                right: None,
            },
            &Node::Number("1"),
        ];

        assert_eq!(
            render(&Node::Row {
                nodes,
                attr: RowAttr::Style(Style::Display)
            }),
            "<mrow displaystyle=\"true\" scriptlevel=\"0\"><mi>x</mi><mo>=</mo><mn>1</mn></mrow>"
        );

        assert_eq!(
            render(&Node::Row {
                nodes,
                attr: RowAttr::Color(0, 0, 0)
            }),
            "<mrow style=\"color:#000000;\"><mi>x</mi><mo>=</mo><mn>1</mn></mrow>"
        );
    }

    #[test]
    fn render_hardcoded_mathml() {
        assert_eq!(render(&Node::HardcodedMathML("<mi>hi</mi>")), "<mi>hi</mi>");
    }

    #[test]
    fn render_sized_paren() {
        assert_eq!(
            render(&Node::SizedParen(
                Size::Scale1,
                symbol::LEFT_PARENTHESIS.as_op(),
            )),
            "<mo maxsize=\"1.2em\" minsize=\"1.2em\">(</mo>"
        );
        assert_eq!(
            render(&Node::SizedParen(
                Size::Scale3,
                symbol::SOLIDUS.as_stretchable_op().unwrap()
            )),
            "<mo maxsize=\"2.047em\" minsize=\"2.047em\" stretchy=\"true\" symmetric=\"true\">/</mo>"
        );
    }

    #[test]
    fn render_text() {
        assert_eq!(render(&Node::Text(None, "hello")), "<mtext>hello</mtext>");
    }

    #[test]
    fn render_table() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator(None),
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
            &Node::RowSeparator(NonZeroU16::new(1)),
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::EquationArray {
                content: &nodes,
                align: Alignment::Centered,
                last_equation_num: NonZeroU16::new(2),
            }),
            "<mtable displaystyle=\"true\" scriptlevel=\"0\" style=\"width: 100%\"><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd><mtd style=\"width: 50%;text-align: right;justify-items: end;\"><mtext>(1)</mtext></mtd></mtr><mtr><mtd style=\"width: 50%\"></mtd><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd><mtd style=\"width: 50%;text-align: right;justify-items: end;\"><mtext>(2)</mtext></mtd></mtr></mtable>"
        );

        assert_eq!(
            render(&Node::EquationArray {
                content: &nodes,
                align: Alignment::Centered,
                last_equation_num: None,
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
            &Node::RowSeparator(None),
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::Array {
                style: None,
                content: &nodes,
                array_spec: &ArraySpec {
                    beginning_line: None,
                    is_sub: false,
                    column_spec: &[
                        ColumnSpec::WithContent(ColumnAlignment::LeftJustified, None),
                        ColumnSpec::WithContent(ColumnAlignment::Centered, None),
                    ],
                },
            }),
            "<mtable><mtr><mtd style=\"text-align: left;justify-items: start;\"><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd style=\"text-align: left;justify-items: start;\"><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_slashed() {
        assert_eq!(
            render(&Node::Slashed(&Node::IdentifierChar(
                'x',
                LetterAttr::Default,
            ))),
            "<mi>x&#x0338;</mi>"
        );
    }

    #[test]
    fn render_multiscript() {
        assert_eq!(
            render(&Node::Multiscript {
                base: &Node::IdentifierChar('x', LetterAttr::Default),
                sub: Some(&Node::Number("1")),
                sup: None,
            }),
            "<mmultiscripts><mi>x</mi><mprescripts/><mn>1</mn><mrow></mrow></mmultiscripts>"
        );
    }

    #[test]
    fn render_text_transform() {
        assert_eq!(
            render(&Node::IdentifierChar('a', LetterAttr::ForcedUpright)),
            "<mpadded><mi mathvariant=\"normal\">a</mi></mpadded>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('a', LetterAttr::ForcedUpright)),
            "<mpadded><mi mathvariant=\"normal\">a</mi></mpadded>"
        );
        assert_eq!(
            render(&Node::IdentifierStr(false, "abc")),
            "<mpadded><mi>abc</mi></mpadded>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝐚', LetterAttr::Default)),
            "<mi>𝐚</mi>"
        );
        assert_eq!(
            render(&Node::IdentifierChar('𝒂', LetterAttr::Default)),
            "<mi>𝒂</mi>"
        );
        assert_eq!(render(&Node::IdentifierStr(true, "𝒂𝒃𝒄")), "<mi>𝒂𝒃𝒄</mi>");
    }

    #[test]
    fn render_enclose() {
        let content = Node::Row {
            nodes: &[
                &Node::IdentifierChar('a', LetterAttr::Default),
                &Node::IdentifierChar('b', LetterAttr::Default),
                &Node::IdentifierChar('c', LetterAttr::Default),
            ],
            attr: RowAttr::None,
        };

        assert_eq!(
            render(&Node::Enclose {
                content: &content,
                notation: Notation::UP_DIAGONAL | Notation::DOWN_DIAGONAL
            }),
            "<menclose notation=\"updiagonalstrike downdiagonalstrike\"><mrow><mi>a</mi><mi>b</mi><mi>c</mi></mrow><mrow class=\"menclose-updiagonalstrike\"></mrow><mrow class=\"menclose-downdiagonalstrike\"></mrow></menclose>"
        );
    }
}
