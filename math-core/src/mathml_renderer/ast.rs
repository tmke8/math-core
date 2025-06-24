use std::fmt::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

use super::attribute::{
    FracAttr, LetterAttr, MathSpacing, MathVariant, OpAttr, RowAttr, Size, StretchMode, Stretchy,
    Style, TextTransform,
};
use super::fmt::new_line_and_indent;
use super::itoa::append_u8_as_hex;
use super::length::{Length, LengthUnit, LengthValue};
use super::symbol::{Fence, MathMLOperator};
use super::table::{Alignment, ArraySpec, ColumnGenerator, LineType, RIGHT_ALIGN};

/// AST node
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Node<'arena> {
    /// `<mn>...</mn>`
    Number(&'arena str),
    /// `<mi>...</mi>` for a single character.
    IdentifierChar(char, LetterAttr),
    StretchableOp(&'static Fence, StretchMode),
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
    IdentifierStr(&'arena str),
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
        open: &'static Fence,
        close: &'static Fence,
        content: &'arena Node<'arena>,
    },
    SizedParen(Size, &'static Fence),
    /// `<mtext>...</mtext>`
    Text(&'arena str),
    /// `<mtable>...</mtable>`
    Table {
        content: &'arena [&'arena Node<'arena>],
        align: Alignment,
        attr: Option<FracAttr>,
        with_numbering: bool,
    },
    Array {
        style: Option<Style>,
        content: &'arena [&'arena Node<'arena>],
        array_spec: &'arena ArraySpec<'arena>,
    },
    /// `<mtd>...</mtd>`
    ColumnSeparator,
    /// `<mtr>...</mtr>`
    RowSeparator,
    Slashed(&'arena Node<'arena>),
    Multiscript {
        base: &'arena Node<'arena>,
        sub: Option<&'arena Node<'arena>>,
        sup: Option<&'arena Node<'arena>>,
    },
    TextTransform {
        tf: MathVariant,
        content: &'arena Node<'arena>,
    },
    CustomCmd {
        predefined: &'arena Node<'arena>,
        args: &'arena [&'arena Node<'arena>],
    },
    CustomCmdArg(usize),
    HardcodedMathML(&'static str),
}

impl PartialEq for &Node<'_> {
    fn eq(&self, other: &&Node<'_>) -> bool {
        std::ptr::eq(*self, *other)
    }
}

macro_rules! writeln_indent {
    ($buf:expr, $indent:expr, $($tail:tt)+) => {
        new_line_and_indent($buf, $indent);
        write!($buf, $($tail)+)?
    };
}

pub struct MathMLEmitter<'converter, 'arena> {
    s: String,
    var: Option<MathVariant>,
    custom_cmd_args: Option<&'arena [&'arena Node<'arena>]>,
    equation_counter: &'converter mut usize,
}

impl<'converter, 'arena> MathMLEmitter<'converter, 'arena> {
    #[inline]
    pub fn new(equation_counter: &'converter mut usize) -> Self {
        Self {
            s: String::new(),
            var: None,
            custom_cmd_args: None,
            equation_counter,
        }
    }

    #[inline]
    pub fn into_inner(self) -> String {
        self.s
    }

    #[inline]
    pub fn push(&mut self, c: char) {
        self.s.push(c);
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.s.push_str(s);
    }

    pub fn emit(&mut self, node: &'arena Node<'arena>, base_indent: usize) -> std::fmt::Result {
        // Compute the indent for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        if !matches!(
            node,
            Node::ColumnSeparator
                | Node::RowSeparator
                | Node::TextTransform { .. }
                | Node::CustomCmd { .. }
                | Node::CustomCmdArg(_)
        ) {
            // Get the base indent out of the way.
            new_line_and_indent(&mut self.s, base_indent);
        }

        match node {
            Node::Number(number) => {
                if let Some(MathVariant::Transform(tf)) = self.var {
                    // We render transformed numbers as identifiers.
                    write!(self.s, "<mi>")?;
                    self.s
                        .extend(number.chars().map(|c| tf.transform(c, false)));
                    write!(self.s, "</mi>")?;
                } else {
                    write!(self.s, "<mn>{number}</mn>")?;
                }
            }
            Node::IdentifierChar(letter, attr) => {
                // The identifier is "normal" if either `is_upright` is set,
                // or the global `self.var` is set to `MathVariant::Normal`.
                let is_normal = matches!(attr, LetterAttr::Upright)
                    || matches!(self.var, Some(MathVariant::Normal));
                // Only set "mathvariant" if we are not transforming the letter.
                if is_normal && !matches!(self.var, Some(MathVariant::Transform(_))) {
                    write!(self.s, "<mi mathvariant=\"normal\">")?;
                } else {
                    write!(self.s, "<mi>")?;
                }
                let c = match self.var {
                    Some(MathVariant::Transform(tf)) => tf.transform(*letter, is_normal),
                    _ => *letter,
                };
                let variant_selector = if matches!(
                    self.var,
                    Some(MathVariant::Transform(TextTransform::ScriptChancery))
                ) {
                    "\u{FE00}"
                } else if matches!(
                    self.var,
                    Some(MathVariant::Transform(TextTransform::ScriptRoundhand))
                ) {
                    "\u{FE01}"
                } else {
                    ""
                };
                write!(self.s, "{c}{variant_selector}</mi>")?;
            }
            Node::TextTransform { content, tf } => {
                let old_var = self.var.replace(*tf);
                self.emit(content, base_indent)?;
                self.var = old_var;
            }
            Node::StretchableOp(op, stretch_mode) => {
                if op.ordinary_spacing() && matches!(stretch_mode, StretchMode::NoStretch) {
                    write!(self.s, "<mi>{}</mi>", char::from(*op))?;
                } else {
                    self.emit_stretchy_op(*stretch_mode, op)?;
                }
            }
            Node::Operator {
                op,
                attr,
                left,
                right,
            } => {
                self.emit_operator_attributes(*attr, *left, *right)?;
                write!(self.s, ">{}</mo>", char::from(op))?;
            }
            Node::PseudoOp {
                attr,
                left,
                right,
                name: text,
            } => {
                self.emit_operator_attributes(*attr, *left, *right)?;
                write!(self.s, ">{text}</mo>")?;
            }
            node @ (Node::IdentifierStr(letters) | Node::Text(letters)) => {
                let (open, close) = match node {
                    Node::IdentifierStr(_) => ("<mi>", "</mi>"),
                    Node::Text(_) => ("<mtext>", "</mtext>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                write!(self.s, "{open}")?;
                match self.var {
                    Some(MathVariant::Transform(tf)) => self
                        .s
                        .extend(letters.chars().map(|c| tf.transform(c, false))),
                    _ => self.s.push_str(letters),
                }
                write!(self.s, "{close}")?;
            }
            Node::Space(space) => {
                write!(self.s, "<mspace width=\"")?;
                space.push_to_string(&mut self.s);
                // Work-around for a Firefox bug that causes "rem" to not be processed correctly
                if matches!(space.unit, LengthUnit::Rem) {
                    write!(self.s, "\" style=\"width:")?;
                    space.push_to_string(&mut self.s);
                }
                write!(self.s, "\"/>")?;
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
                write!(self.s, "{open}")?;
                self.emit(first, child_indent)?;
                self.emit(second, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "{close}");
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
                write!(self.s, "{open}")?;
                self.emit(first, child_indent)?;
                self.emit(second, child_indent)?;
                self.emit(third, child_indent)?;
                writeln_indent!(&mut self.s, base_indent, "{close}");
            }
            Node::Multiscript { base, sub, sup } => {
                write!(self.s, "<mmultiscripts>")?;
                self.emit(base, child_indent)?;
                writeln_indent!(&mut self.s, child_indent, "<mprescripts/>");
                if let Some(sub) = sub {
                    self.emit(sub, child_indent)?;
                } else {
                    writeln_indent!(&mut self.s, child_indent, "<mrow></mrow>");
                }
                if let Some(sup) = sup {
                    self.emit(sup, child_indent)?;
                } else {
                    writeln_indent!(&mut self.s, child_indent, "<mrow></mrow>");
                }
                writeln_indent!(&mut self.s, base_indent, "</mmultiscripts>");
            }
            Node::OverOp(op, attr, target) => {
                write!(self.s, "<mover accent=\"true\">")?;
                self.emit(target, child_indent)?;
                writeln_indent!(&mut self.s, child_indent, "<mo");
                if let Some(attr) = attr {
                    write!(self.s, "{}", <&str>::from(attr))?;
                }
                write!(self.s, ">{}</mo>", char::from(op))?;
                writeln_indent!(&mut self.s, base_indent, "</mover>");
            }
            Node::UnderOp(op, target) => {
                write!(self.s, "<munder accent=\"true\">")?;
                self.emit(target, child_indent)?;
                writeln_indent!(&mut self.s, child_indent, "<mo>{}</mo>", char::from(op));
                writeln_indent!(&mut self.s, base_indent, "</munder>");
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
                let lt = Length::from_parts(*line_length, *line_unit);
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
            Node::Row { nodes, attr: style } => {
                match style {
                    RowAttr::None => {
                        write!(self.s, "<mrow>")?;
                    }
                    RowAttr::Style(style) => {
                        write!(self.s, "<mrow{}>", <&str>::from(style))?;
                    }
                    RowAttr::Color(r, g, b) => {
                        write!(self.s, "<mrow style=\"color:#")?;
                        append_u8_as_hex(&mut self.s, *r);
                        append_u8_as_hex(&mut self.s, *g);
                        append_u8_as_hex(&mut self.s, *b);
                        write!(self.s, ";\">")?;
                    }
                }
                for node in nodes.iter() {
                    self.emit(node, child_indent)?;
                }
                writeln_indent!(&mut self.s, base_indent, "</mrow>");
            }
            Node::Fenced {
                open,
                close,
                content,
                style,
            } => {
                match style {
                    Some(style) => write!(self.s, "<mrow{}>", <&str>::from(style))?,
                    None => write!(self.s, "<mrow>")?,
                };
                new_line_and_indent(&mut self.s, child_indent);
                self.emit_stretchy_op(StretchMode::Fence, open)?;
                self.emit(content, child_indent)?;
                new_line_and_indent(&mut self.s, child_indent);
                self.emit_stretchy_op(StretchMode::Fence, close)?;
                writeln_indent!(&mut self.s, base_indent, "</mrow>");
            }
            Node::SizedParen(size, paren) => {
                write!(
                    self.s,
                    "<mo maxsize=\"{}\" minsize=\"{}\"",
                    <&str>::from(size),
                    <&str>::from(size)
                )?;
                if !matches!(paren.stretchy(), Stretchy::Always) {
                    write!(self.s, " stretchy=\"true\" symmetric=\"true\"")?;
                }
                write!(self.s, ">{}</mo>", char::from(*paren))?;
            }
            Node::Slashed(node) => match node {
                Node::IdentifierChar(x, attr) => {
                    if matches!(attr, LetterAttr::Upright)
                        || matches!(self.var, Some(MathVariant::Normal))
                    {
                        write!(self.s, "<mi mathvariant=\"normal\">{x}&#x0338;</mi>")?;
                    } else {
                        write!(self.s, "<mi>{x}&#x0338;</mi>")?;
                    }
                }
                Node::Operator { op, .. } => {
                    write!(self.s, "<mo>{}&#x0338;</mo>", char::from(op))?;
                }
                n => self.emit(n, base_indent)?,
            },
            Node::Table {
                content,
                align,
                attr,
                with_numbering,
            } => {
                let mtd_opening = ColumnGenerator::new_predefined(*align);
                let with_numbering = *with_numbering;

                write!(self.s, "<mtable")?;
                if let Some(attr) = attr {
                    write!(self.s, "{}", <&str>::from(attr))?;
                }
                if with_numbering {
                    write!(self.s, r#" style="width: 100%""#)?;
                }
                write!(self.s, ">")?;
                self.emit_table(
                    base_indent,
                    child_indent,
                    content,
                    mtd_opening,
                    with_numbering,
                )?;
            }
            Node::Array {
                style,
                content,
                array_spec,
            } => {
                let mtd_opening = ColumnGenerator::new_custom(array_spec);
                write!(self.s, "<mtable")?;
                match array_spec.beginning_line {
                    Some(LineType::Solid) => {
                        write!(self.s, " style=\"border-left: 0.05em solid currentcolor\"")?;
                    }
                    Some(LineType::Dashed) => {
                        write!(self.s, " style=\"border-left: 0.05em dashed currentcolor\"")?;
                    }
                    _ => (),
                }
                if let Some(style) = style {
                    write!(self.s, "{}", <&str>::from(style))?;
                }
                write!(self.s, ">")?;
                self.emit_table(base_indent, child_indent, content, mtd_opening, false)?;
            }
            Node::ColumnSeparator | Node::RowSeparator => (),
            Node::CustomCmd { predefined, args } => {
                let old_args = self.custom_cmd_args.replace(args);
                self.emit(predefined, base_indent)?;
                self.custom_cmd_args = old_args;
            }
            Node::CustomCmdArg(index) => {
                if let Some(arg) = self
                    .custom_cmd_args
                    .as_ref()
                    .and_then(|args| args.get(*index))
                {
                    self.emit(arg, base_indent)?;
                }
            }
            Node::HardcodedMathML(mathml) => {
                write!(self.s, "{mathml}")?;
            }
        };
        Ok(())
    }

    fn emit_operator_attributes(
        &mut self,
        attr: Option<OpAttr>,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    ) -> std::fmt::Result {
        match attr {
            Some(attributes) => write!(self.s, "<mo{}", <&str>::from(attributes))?,
            None => write!(self.s, "<mo")?,
        };
        match (left, right) {
            (Some(left), Some(right)) => {
                write!(
                    self.s,
                    " lspace=\"{}\" rspace=\"{}\"",
                    <&str>::from(left),
                    <&str>::from(right)
                )?;
            }
            (Some(left), None) => {
                write!(self.s, " lspace=\"{}\"", <&str>::from(left))?;
            }
            (None, Some(right)) => {
                write!(self.s, " rspace=\"{}\"", <&str>::from(right))?;
            }
            _ => {}
        };
        Ok(())
    }

    fn emit_table(
        &mut self,
        base_indent: usize,
        child_indent: usize,
        content: &'arena [&Node<'arena>],
        mut col_gen: ColumnGenerator,
        with_numbering: bool,
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
        writeln_indent!(&mut self.s, child_indent, "<mtr>");
        if with_numbering {
            // Add a dummy column to help keep everything centered.
            writeln_indent!(
                &mut self.s,
                child_indent2,
                r#"<mtd style="width: 50%"></mtd>"#
            );
        }
        col_gen.write_next_mtd(&mut self.s, child_indent2)?;
        for node in content.iter() {
            match node {
                Node::ColumnSeparator => {
                    writeln_indent!(&mut self.s, child_indent2, "</mtd>");
                    col_gen.write_next_mtd(&mut self.s, child_indent2)?;
                }
                Node::RowSeparator => {
                    writeln_indent!(&mut self.s, child_indent2, "</mtd>");
                    if with_numbering {
                        self.write_equation_num(child_indent, child_indent2)?;
                    }
                    writeln_indent!(&mut self.s, child_indent, "</mtr>");
                    writeln_indent!(&mut self.s, child_indent, "<mtr>");
                    col_gen.reset_columns();
                    if with_numbering {
                        // Add a dummy column to help keep everything centered.
                        writeln_indent!(
                            &mut self.s,
                            child_indent2,
                            r#"<mtd style="width: 50%"></mtd>"#
                        );
                    }
                    col_gen.write_next_mtd(&mut self.s, child_indent2)?;
                }
                node => {
                    self.emit(node, child_indent3)?;
                }
            }
        }
        writeln_indent!(&mut self.s, child_indent2, "</mtd>");
        if with_numbering {
            self.write_equation_num(child_indent2, child_indent3)?;
        }
        writeln_indent!(&mut self.s, child_indent, "</mtr>");
        writeln_indent!(&mut self.s, base_indent, "</mtable>");
        Ok(())
    }

    fn write_equation_num(
        &mut self,
        child_indent2: usize,
        child_indent3: usize,
    ) -> Result<(), std::fmt::Error> {
        *self.equation_counter += 1;
        writeln_indent!(
            &mut self.s,
            child_indent2,
            r#"<mtd style="width: 50%;{}">"#,
            RIGHT_ALIGN
        );
        writeln_indent!(
            &mut self.s,
            child_indent3,
            "<mtext>({})</mtext>",
            self.equation_counter
        );
        writeln_indent!(&mut self.s, child_indent2, "</mtd>");
        Ok(())
    }

    fn emit_stretchy_op(&mut self, stretch_mode: StretchMode, op: &Fence) -> std::fmt::Result {
        match (stretch_mode, op.stretchy()) {
            (StretchMode::Fence, Stretchy::Never | Stretchy::Inconsistent)
            | (
                StretchMode::Middle,
                Stretchy::PrePostfix | Stretchy::Inconsistent | Stretchy::Never,
            ) => {
                write!(self.s, "<mo stretchy=\"true\">")?;
            }
            (
                StretchMode::NoStretch,
                Stretchy::Always | Stretchy::PrePostfix | Stretchy::Inconsistent,
            ) => {
                write!(self.s, "<mo stretchy=\"false\">")?;
            }
            _ => {
                write!(self.s, "<mo>")?;
            }
        }
        if char::from(op) != '\0' {
            write!(self.s, "{}", char::from(op))?;
        }
        write!(self.s, "</mo>")?;
        Ok(())
    }
}

// impl Default for MathMLEmitter<'static> {
//     fn default() -> Self {
//         Self::new()
//     }
// }

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
        let mut equation_counter = 0;
        let mut emitter = MathMLEmitter::new(&mut equation_counter);
        emitter.emit(node, 0).unwrap();
        emitter.into_inner()
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
            render(&Node::IdentifierChar('Γ', LetterAttr::Upright)),
            "<mi mathvariant=\"normal\">Γ</mi>"
        );

        let mut equation_counter = 0;
        let mut emitter = MathMLEmitter::new(&mut equation_counter);
        emitter.var = Some(MathVariant::Transform(TextTransform::ScriptRoundhand));
        emitter
            .emit(&Node::IdentifierChar('L', LetterAttr::Default), 0)
            .unwrap();
        assert_eq!(emitter.into_inner(), "<mi>ℒ︁</mi>");
    }

    #[test]
    fn render_operator_with_spacing() {
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.into(),
                attr: None,
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::FourMu),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0.2222em\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::COLON.into(),
                attr: None,
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::Zero),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0\">:</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::IDENTICAL_TO.into(),
                attr: None,
                left: Some(MathSpacing::Zero),
                right: None,
            }),
            "<mo lspace=\"0\">≡</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::PLUS_SIGN.into(),
                attr: Some(OpAttr::FormPrefix),
                left: None,
                right: None,
            }),
            "<mo form=\"prefix\">+</mo>"
        );
        assert_eq!(
            render(&Node::Operator {
                op: symbol::N_ARY_SUMMATION.into(),
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
        assert_eq!(render(&Node::IdentifierStr("sin")), "<mi>sin</mi>");
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
                symbol::MACRON.into(),
                Some(OpAttr::StretchyFalse),
                &Node::IdentifierChar('x', LetterAttr::Default),
            )),
            "<mover accent=\"true\"><mi>x</mi><mo stretchy=\"false\">¯</mo></mover>"
        );
        assert_eq!(
            render(&Node::OverOp(
                symbol::OVERLINE.into(),
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
                symbol::LOW_LINE.into(),
                &Node::IdentifierChar('x', LetterAttr::Default),
            )),
            "<munder accent=\"true\"><mi>x</mi><mo>_</mo></munder>"
        );
    }

    #[test]
    fn render_overset() {
        assert_eq!(
            render(&Node::Overset {
                symbol: &Node::Operator {
                    op: symbol::EXCLAMATION_MARK.into(),
                    attr: None,
                    left: None,
                    right: None
                },
                target: &Node::Operator {
                    op: symbol::EQUALS_SIGN.into(),
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
                op: symbol::EQUALS_SIGN.into(),
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
            render(&Node::SizedParen(Size::Scale1, symbol::LEFT_PARENTHESIS,)),
            "<mo maxsize=\"1.2em\" minsize=\"1.2em\">(</mo>"
        );
        assert_eq!(
            render(&Node::SizedParen(Size::Scale3, symbol::SOLIDUS,)),
            "<mo maxsize=\"2.047em\" minsize=\"2.047em\" stretchy=\"true\" symmetric=\"true\">/</mo>"
        );
    }

    #[test]
    fn render_text() {
        assert_eq!(render(&Node::Text("hello")), "<mtext>hello</mtext>");
    }

    #[test]
    fn render_table() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator,
            &Node::Number("3"),
            &Node::ColumnSeparator,
            &Node::Number("4"),
        ];

        assert_eq!(
            render(&Node::Table {
                content: &nodes,
                align: Alignment::Centered,
                attr: None,
                with_numbering: false,
            }),
            "<mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_array() {
        let nodes = [
            &Node::Number("1"),
            &Node::ColumnSeparator,
            &Node::Number("2"),
            &Node::RowSeparator,
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
            "<mtable><mtr><mtd style=\"text-align: -webkit-left;text-align: -moz-left;\"><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd style=\"text-align: -webkit-left;text-align: -moz-left;\"><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_slashed() {
        assert_eq!(
            render(&Node::Slashed(&Node::IdentifierChar(
                'x',
                LetterAttr::Default
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
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::IdentifierChar('a', LetterAttr::Upright),
            }),
            "<mi mathvariant=\"normal\">a</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::IdentifierChar('a', LetterAttr::Default),
            }),
            "<mi mathvariant=\"normal\">a</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::IdentifierStr("abc"),
            }),
            "<mi>abc</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::PseudoOp {
                    name: "abc",
                    attr: None,
                    left: Some(MathSpacing::ThreeMu),
                    right: Some(MathSpacing::ThreeMu),
                }
            }),
            "<mo lspace=\"0.1667em\" rspace=\"0.1667em\">abc</mo>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::IdentifierChar('a', LetterAttr::Upright),
            }),
            "<mi>𝐚</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::IdentifierChar('a', LetterAttr::Default),
            }),
            "<mi>𝒂</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::IdentifierStr("abc"),
            }),
            "<mi>𝒂𝒃𝒄</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::PseudoOp {
                    name: "abc",
                    attr: None,
                    left: Some(MathSpacing::ThreeMu),
                    right: Some(MathSpacing::ThreeMu),
                },
            }),
            "<mo lspace=\"0.1667em\" rspace=\"0.1667em\">abc</mo>"
        );
    }
}
