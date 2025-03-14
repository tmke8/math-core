use std::mem;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::attribute::{
    Align, FracAttr, MathSpacing, MathVariant, OpAttr, RowAttr, Size, StretchMode, Stretchy, Style,
};
use crate::itoa::append_u8_as_hex;
use crate::length::SpecifiedLength;
use crate::symbol::{Op, ParenOp};

/// AST node
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Node<'arena> {
    Number(&'arena str),
    SingleLetterIdent(char, bool),
    Operator(Op, Option<OpAttr>),
    StretchableOp(&'static ParenOp, StretchMode),
    OpGreaterThan,
    OpLessThan,
    OpAmpersand,
    OperatorWithSpacing {
        op: Op,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    MultiLetterIdent(&'arena str),
    CollectedLetters(&'arena str),
    Space(&'static str),
    Subscript {
        target: &'arena Node<'arena>,
        symbol: &'arena Node<'arena>,
    },
    Superscript {
        target: &'arena Node<'arena>,
        symbol: &'arena Node<'arena>,
    },
    SubSup {
        target: &'arena Node<'arena>,
        sub: &'arena Node<'arena>,
        sup: &'arena Node<'arena>,
    },
    OverOp(Op, Option<OpAttr>, &'arena Node<'arena>),
    UnderOp(Op, &'arena Node<'arena>),
    Overset {
        symbol: &'arena Node<'arena>,
        target: &'arena Node<'arena>,
    },
    Underset {
        symbol: &'arena Node<'arena>,
        target: &'arena Node<'arena>,
    },
    UnderOver {
        target: &'arena Node<'arena>,
        under: &'arena Node<'arena>,
        over: &'arena Node<'arena>,
    },
    Sqrt(&'arena Node<'arena>),
    Root(&'arena Node<'arena>, &'arena Node<'arena>),
    Frac {
        /// Numerator
        num: &'arena Node<'arena>,
        /// Denominator
        den: &'arena Node<'arena>,
        /// Line thickness
        lt: Option<SpecifiedLength>,
        attr: Option<FracAttr>,
    },
    Row {
        nodes: &'arena [&'arena Node<'arena>],
        attr: RowAttr,
    },
    Fenced {
        style: Option<Style>,
        open: &'static ParenOp,
        close: &'static ParenOp,
        content: &'arena Node<'arena>,
    },
    SizedParen(Size, &'static ParenOp),
    Text(&'arena str),
    Table {
        content: &'arena [&'arena Node<'arena>],
        align: Align,
        attr: Option<FracAttr>,
    },
    ColumnSeparator,
    RowSeparator,
    Slashed(&'arena Node<'arena>),
    Multiscript {
        base: &'arena Node<'arena>,
        sub: &'arena Node<'arena>,
    },
    TextTransform {
        tf: MathVariant,
        content: &'arena Node<'arena>,
    },
    CustomCmd {
        predefined: &'static Node<'static>,
        args: &'arena [&'arena Node<'arena>],
    },
    CustomCmdArg(usize),
    HardcodedMathML(&'static str),
}

impl PartialEq for &'static Node<'static> {
    fn eq(&self, other: &&'static Node<'static>) -> bool {
        std::ptr::eq(*self, *other)
    }
}

const INDENT: &str = "    ";

macro_rules! push {
    ($buf:expr, @ $c:expr $(,)?) => {{
        $buf.push($c.into());
    }};
    ($buf:expr, $s:expr $(,)?) => {{
        $buf.push_str($s.as_ref());
    }};
    ($buf:expr, @ $c:expr, $($tail:tt)+) => {{
        $buf.push($c.into());
        push!($buf, $($tail)+)
    }};
    ($buf:expr, $s:expr, $($tail:tt)+) => {{
        $buf.push_str($s.as_ref());
        push!($buf, $($tail)+)
    }};
}

macro_rules! pushln {
    ($buf:expr, $indent:expr, $($tail:tt)+) => {
        new_line_and_indent($buf, $indent);
        push!($buf, $($tail)+)
    };
}

pub struct MathMLEmitter<'arena> {
    s: String,
    var: Option<MathVariant>,
    custom_cmd_args: Option<&'arena [&'arena Node<'arena>]>,
}

impl<'arena> MathMLEmitter<'arena> {
    #[inline]
    pub fn new() -> Self {
        Self {
            s: String::new(),
            var: None,
            custom_cmd_args: None,
        }
    }

    #[inline]
    pub fn into_inner(self) -> String {
        self.s
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.s
    }

    #[inline]
    pub fn clear(&mut self) {
        self.s.clear();
    }

    #[inline]
    pub fn push(&mut self, c: char) {
        self.s.push(c);
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.s.push_str(s);
    }

    pub fn emit(&mut self, node: &'arena Node<'arena>, base_indent: usize) {
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
                    push!(self.s, "<mi>");
                    self.s
                        .extend(number.chars().map(|c| tf.transform(c, false)));
                    push!(self.s, "</mi>");
                } else {
                    push!(self.s, "<mn>", number, "</mn>");
                }
            }
            Node::SingleLetterIdent(letter, is_normal) => {
                // The identifier is "normal" if either `is_normal` is set,
                // or the global `self.var` is set to `MathVariant::Normal`.
                let is_normal = *is_normal || matches!(self.var, Some(MathVariant::Normal));
                // Only set "mathvariant" if we are not transforming the letter.
                if is_normal && !matches!(self.var, Some(MathVariant::Transform(_))) {
                    push!(self.s, "<mi mathvariant=\"normal\">");
                } else {
                    push!(self.s, "<mi>");
                }
                let c = match self.var {
                    Some(MathVariant::Transform(tf)) => tf.transform(*letter, is_normal),
                    _ => *letter,
                };
                push!(self.s, @c, "</mi>");
            }
            Node::TextTransform { content, tf } => {
                let old_var = mem::replace(&mut self.var, Some(*tf));
                self.emit(content, base_indent);
                self.var = old_var;
            }
            Node::Operator(op, attributes) => {
                match attributes {
                    Some(attributes) => push!(self.s, "<mo", attributes, ">"),
                    None => push!(self.s, "<mo>"),
                }
                push!(self.s, @op, "</mo>");
            }
            Node::StretchableOp(op, stretch_mode) => {
                if op.ordinary_spacing() && matches!(stretch_mode, StretchMode::NoStretch) {
                    push!(self.s, "<mi>", @*op, "</mi>");
                } else {
                    self.emit_stretchy_op(*stretch_mode, op);
                }
            }
            node @ (Node::OpGreaterThan | Node::OpLessThan | Node::OpAmpersand) => {
                let op = match node {
                    Node::OpGreaterThan => "&gt;",
                    Node::OpLessThan => "&lt;",
                    Node::OpAmpersand => "&amp;",
                    _ => unreachable!(),
                };
                push!(self.s, "<mo>", op, "</mo>");
            }
            Node::OperatorWithSpacing { op, left, right } => {
                match (left, right) {
                    (Some(left), Some(right)) => {
                        push!(self.s, "<mo lspace=\"", left, "\" rspace=\"", right, "\"",)
                    }
                    (Some(left), None) => {
                        push!(self.s, "<mo lspace=\"", left, "\"")
                    }
                    (None, Some(right)) => {
                        push!(self.s, "<mo rspace=\"", right, "\"")
                    }
                    (None, None) => self.s.push_str("<mo"),
                }
                push!(self.s, ">", @op, "</mo>");
            }
            Node::MultiLetterIdent(letters) => {
                push!(self.s, "<mi>", letters, "</mi>");
            }
            node @ (Node::CollectedLetters(letters) | Node::Text(letters)) => {
                let (open, close) = match node {
                    Node::CollectedLetters(_) => ("<mi>", "</mi>"),
                    Node::Text(_) => ("<mtext>", "</mtext>"),
                    // Compiler is able to infer that this is unreachable.
                    _ => unreachable!(),
                };
                push!(self.s, open);
                match self.var {
                    Some(MathVariant::Transform(tf)) => self
                        .s
                        .extend(letters.chars().map(|c| tf.transform(c, false))),
                    _ => self.s.push_str(letters),
                }
                push!(self.s, close);
            }
            Node::Space(space) => push!(self.s, "<mspace width=\"", space, "em\"/>"),
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
                push!(self.s, open);
                self.emit(first, child_indent);
                self.emit(second, child_indent);
                pushln!(&mut self.s, base_indent, close);
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
                push!(self.s, open);
                self.emit(first, child_indent);
                self.emit(second, child_indent);
                self.emit(third, child_indent);
                pushln!(&mut self.s, base_indent, close);
            }
            Node::Multiscript { base, sub } => {
                push!(self.s, "<mmultiscripts>");
                self.emit(base, child_indent);
                pushln!(&mut self.s, child_indent, "<mprescripts/>");
                self.emit(sub, child_indent);
                pushln!(&mut self.s, child_indent, "<mrow></mrow>");
                pushln!(&mut self.s, base_indent, "</mmultiscripts>");
            }
            Node::OverOp(op, attr, target) => {
                push!(self.s, "<mover>");
                self.emit(target, child_indent);
                pushln!(&mut self.s, child_indent, "<mo accent=\"true\"");
                if let Some(attr) = attr {
                    push!(self.s, attr);
                }
                push!(self.s, ">", @op, "</mo>");
                pushln!(&mut self.s, base_indent, "</mover>");
            }
            Node::UnderOp(op, target) => {
                push!(self.s, "<munder>");
                self.emit(target, child_indent);
                pushln!(&mut self.s, child_indent, "<mo accent=\"true\">", @op, "</mo>");
                pushln!(&mut self.s, base_indent, "</munder>");
            }
            Node::Sqrt(content) => {
                push!(self.s, "<msqrt>");
                self.emit(content, child_indent);
                pushln!(&mut self.s, base_indent, "</msqrt>");
            }
            Node::Frac { num, den, lt, attr } => {
                push!(self.s, "<mfrac");
                if let Some(lt) = lt {
                    push!(self.s, " linethickness=\"");
                    lt.push_to_string(&mut self.s);
                    push!(self.s, "\"");
                }
                if let Some(style) = attr {
                    push!(self.s, style);
                }
                push!(self.s, ">");
                self.emit(num, child_indent);
                self.emit(den, child_indent);
                pushln!(&mut self.s, base_indent, "</mfrac>");
            }
            Node::Row { nodes, attr: style } => {
                match style {
                    RowAttr::None => push!(self.s, "<mrow>"),
                    RowAttr::Style(style) => push!(self.s, "<mrow", style, ">"),
                    RowAttr::Color(r, g, b) => {
                        push!(self.s, "<mrow style=\"color:#");
                        append_u8_as_hex(&mut self.s, *r);
                        append_u8_as_hex(&mut self.s, *g);
                        append_u8_as_hex(&mut self.s, *b);
                        push!(self.s, ";\">");
                    }
                }
                for node in nodes.iter() {
                    self.emit(node, child_indent);
                }
                pushln!(&mut self.s, base_indent, "</mrow>");
            }
            Node::Fenced {
                open,
                close,
                content,
                style,
            } => {
                match style {
                    Some(style) => push!(self.s, "<mrow", style, ">"),
                    None => push!(self.s, "<mrow>"),
                }
                new_line_and_indent(&mut self.s, child_indent);
                self.emit_stretchy_op(StretchMode::Fence, open);
                self.emit(content, child_indent);
                new_line_and_indent(&mut self.s, child_indent);
                self.emit_stretchy_op(StretchMode::Fence, close);
                pushln!(&mut self.s, base_indent, "</mrow>");
            }
            Node::SizedParen(size, paren) => {
                push!(self.s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\"");
                if !matches!(paren.stretchy(), Stretchy::Always) {
                    push!(self.s, " stretchy=\"true\" symmetric=\"true\"");
                }
                push!(self.s, ">", @*paren, "</mo>");
            }
            Node::Slashed(node) => match node {
                Node::SingleLetterIdent(x, is_normal) => {
                    if *is_normal || matches!(self.var, Some(MathVariant::Normal)) {
                        push!(self.s, "<mi mathvariant=\"normal\">", @*x, "&#x0338;</mi>");
                    } else {
                        push!(self.s, "<mi>", @*x, "&#x0338;</mi>");
                    }
                }
                Node::Operator(x, _) => {
                    push!(self.s, "<mo>", @x, "&#x0338;</mo>");
                }
                n => self.emit(n, base_indent),
            },
            Node::Table {
                content,
                align,
                attr,
            } => {
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
                let odd_col = match align {
                    Align::Center => "<mtd>",
                    Align::Left => {
                        r#"<mtd style="text-align: -webkit-left; text-align: -moz-left; padding-right: 0">"#
                    }
                    Align::Alternating => {
                        r#"<mtd style="text-align: -webkit-right; text-align: -moz-right; padding-right: 0">"#
                    }
                };
                let even_col = match align {
                    Align::Center => "<mtd>",
                    Align::Left => {
                        "<mtd style=\"text-align: -webkit-left; text-align: -moz-left; padding-right: 0; padding-left: 1em\">"
                    }
                    Align::Alternating => {
                        "<mtd style=\"text-align: -webkit-left; text-align: -moz-left; padding-left: 0\">"
                    }
                };

                let mut col: usize = 1;
                push!(self.s, "<mtable");
                if let Some(attr) = attr {
                    push!(self.s, attr);
                }
                push!(self.s, ">");
                pushln!(&mut self.s, child_indent, "<mtr>");
                pushln!(&mut self.s, child_indent2, odd_col);
                for node in content.iter() {
                    match node {
                        Node::ColumnSeparator => {
                            pushln!(&mut self.s, child_indent2, "</mtd>");
                            col += 1;
                            pushln!(
                                &mut self.s,
                                child_indent2,
                                if col % 2 == 0 { even_col } else { odd_col }
                            );
                        }
                        Node::RowSeparator => {
                            pushln!(&mut self.s, child_indent2, "</mtd>");
                            pushln!(&mut self.s, child_indent, "</mtr>");
                            pushln!(&mut self.s, child_indent, "<mtr>");
                            pushln!(&mut self.s, child_indent2, odd_col);
                            col = 1;
                        }
                        node => {
                            self.emit(node, child_indent3);
                        }
                    }
                }
                pushln!(&mut self.s, child_indent2, "</mtd>");
                pushln!(&mut self.s, child_indent, "</mtr>");
                pushln!(&mut self.s, base_indent, "</mtable>");
            }
            Node::ColumnSeparator | Node::RowSeparator => (),
            Node::CustomCmd { predefined, args } => {
                let old_args = mem::replace(&mut self.custom_cmd_args, Some(args));
                self.emit(predefined, base_indent);
                self.custom_cmd_args = old_args;
            }
            Node::CustomCmdArg(index) => {
                if let Some(arg) = self
                    .custom_cmd_args
                    .as_ref()
                    .and_then(|args| args.get(*index))
                {
                    self.emit(arg, base_indent);
                }
            }
            Node::HardcodedMathML(mathml) => {
                push!(self.s, mathml);
            }
        }
    }

    fn emit_stretchy_op(&mut self, stretch_mode: StretchMode, op: &ParenOp) {
        match (stretch_mode, op.stretchy()) {
            (StretchMode::Fence, Stretchy::Never | Stretchy::Inconsistent)
            | (
                StretchMode::Middle,
                Stretchy::PrePostfix | Stretchy::Inconsistent | Stretchy::Never,
            ) => {
                push!(self.s, "<mo stretchy=\"true\">")
            }
            (
                StretchMode::NoStretch,
                Stretchy::Always | Stretchy::PrePostfix | Stretchy::Inconsistent,
            ) => {
                push!(self.s, "<mo stretchy=\"false\">")
            }
            _ => push!(self.s, "<mo>"),
        }
        if char::from(op) != '\0' {
            push!(self.s, @op);
        }
        push!(self.s, "</mo>");
    }
}

impl Default for MathMLEmitter<'static> {
    fn default() -> Self {
        Self::new()
    }
}

fn new_line_and_indent(s: &mut String, indent_num: usize) {
    if indent_num > 0 {
        s.push('\n');
    }
    for _ in 0..indent_num {
        s.push_str(INDENT);
    }
}

#[cfg(test)]
mod tests {
    use super::{MathMLEmitter, Node};
    use crate::attribute::{
        FracAttr, MathSpacing, MathVariant, OpAttr, RowAttr, Style, TextTransform,
    };
    use crate::length::{AbsoluteLength, FontRelativeLength};
    use crate::symbol;

    pub fn render<'a, 'b>(node: &'a Node<'b>) -> String
    where
        'a: 'b,
    {
        let mut emitter = MathMLEmitter::new();
        emitter.emit(node, 0);
        emitter.into_inner()
    }

    #[test]
    fn render_number() {
        assert_eq!(render(&Node::Number("3.14")), "<mn>3.14</mn>");
    }

    #[test]
    fn render_single_letter_ident() {
        assert_eq!(render(&Node::SingleLetterIdent('x', false)), "<mi>x</mi>");
        assert_eq!(
            render(&Node::SingleLetterIdent('Γ', true)),
            "<mi mathvariant=\"normal\">Γ</mi>"
        );
    }

    #[test]
    fn render_operator() {
        assert_eq!(
            render(&Node::Operator(symbol::EQUALS_SIGN.into(), None)),
            "<mo>=</mo>"
        );
        assert_eq!(
            render(&Node::Operator(
                symbol::N_ARY_SUMMATION.into(),
                Some(OpAttr::NoMovableLimits)
            )),
            "<mo movablelimits=\"false\">∑</mo>"
        );
    }

    #[test]
    fn render_op_greater_than() {
        assert_eq!(render(&Node::OpGreaterThan), "<mo>&gt;</mo>");
    }

    #[test]
    fn render_op_less_than() {
        assert_eq!(render(&Node::OpLessThan), "<mo>&lt;</mo>");
    }

    #[test]
    fn render_op_ampersand() {
        assert_eq!(render(&Node::OpAmpersand), "<mo>&amp;</mo>");
    }

    #[test]
    fn render_operator_with_spacing() {
        assert_eq!(
            render(&Node::OperatorWithSpacing {
                op: symbol::COLON.into(),
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::FourMu),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0.2222em\">:</mo>"
        );
        assert_eq!(
            render(&Node::OperatorWithSpacing {
                op: symbol::COLON.into(),
                left: Some(MathSpacing::FourMu),
                right: Some(MathSpacing::Zero),
            }),
            "<mo lspace=\"0.2222em\" rspace=\"0em\">:</mo>"
        );
        assert_eq!(
            render(&Node::OperatorWithSpacing {
                op: symbol::IDENTICAL_TO.into(),
                left: Some(MathSpacing::Zero),
                right: None,
            }),
            "<mo lspace=\"0em\">≡</mo>"
        );
    }

    #[test]
    fn render_multi_letter_ident() {
        assert_eq!(render(&Node::MultiLetterIdent("sin")), "<mi>sin</mi>");
    }

    #[test]
    fn render_collected_letters() {
        assert_eq!(render(&Node::CollectedLetters("sin")), "<mi>sin</mi>");
    }

    #[test]
    fn render_space() {
        assert_eq!(render(&Node::Space("1")), "<mspace width=\"1em\"/>");
    }

    #[test]
    fn render_subscript() {
        assert_eq!(
            render(&Node::Subscript {
                target: &Node::SingleLetterIdent('x', false),
                symbol: &Node::Number("2"),
            }),
            "<msub><mi>x</mi><mn>2</mn></msub>"
        );
    }

    #[test]
    fn render_superscript() {
        assert_eq!(
            render(&Node::Superscript {
                target: &Node::SingleLetterIdent('x', false),
                symbol: &Node::Number("2"),
            }),
            "<msup><mi>x</mi><mn>2</mn></msup>"
        );
    }

    #[test]
    fn render_sub_sup() {
        assert_eq!(
            render(&Node::SubSup {
                target: &Node::SingleLetterIdent('x', false),
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
                &Node::SingleLetterIdent('x', false),
            )),
            "<mover><mi>x</mi><mo accent=\"true\" stretchy=\"false\">¯</mo></mover>"
        );
        assert_eq!(
            render(&Node::OverOp(
                symbol::OVERLINE.into(),
                None,
                &Node::SingleLetterIdent('x', false),
            )),
            "<mover><mi>x</mi><mo accent=\"true\">‾</mo></mover>"
        );
    }

    #[test]
    fn render_under_op() {
        assert_eq!(
            render(&Node::UnderOp(
                symbol::LOW_LINE.into(),
                &Node::SingleLetterIdent('x', false),
            )),
            "<munder><mi>x</mi><mo accent=\"true\">_</mo></munder>"
        );
    }

    #[test]
    fn render_overset() {
        assert_eq!(
            render(&Node::Overset {
                symbol: &Node::Operator(symbol::EXCLAMATION_MARK.into(), None),
                target: &Node::Operator(symbol::EQUALS_SIGN.into(), None),
            }),
            "<mover><mo>=</mo><mo>!</mo></mover>"
        );
    }

    #[test]
    fn render_underset() {
        assert_eq!(
            render(&Node::Underset {
                symbol: &Node::SingleLetterIdent('θ', false),
                target: &Node::MultiLetterIdent("min"),
            }),
            "<munder><mi>min</mi><mi>θ</mi></munder>"
        );
    }

    #[test]
    fn render_under_over() {
        assert_eq!(
            render(&Node::UnderOver {
                target: &Node::SingleLetterIdent('x', false),
                under: &Node::Number("1"),
                over: &Node::Number("2"),
            }),
            "<munderover><mi>x</mi><mn>1</mn><mn>2</mn></munderover>"
        );
    }

    #[test]
    fn render_sqrt() {
        assert_eq!(
            render(&Node::Sqrt(&Node::SingleLetterIdent('x', false))),
            "<msqrt><mi>x</mi></msqrt>"
        );
    }

    #[test]
    fn render_root() {
        assert_eq!(
            render(&Node::Root(
                &Node::Number("3"),
                &Node::SingleLetterIdent('x', false),
            )),
            "<mroot><mi>x</mi><mn>3</mn></mroot>"
        );
    }

    #[test]
    fn render_frac() {
        let num = &Node::Number("1");
        let den = &Node::Number("2");
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: None,
                attr: None,
            }),
            "<mfrac><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: Some(AbsoluteLength::from_pt(-1).into()),
                attr: None,
            }),
            "<mfrac linethickness=\"-1pt\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: Some(FontRelativeLength::from_em(1).into()),
                attr: None,
            }),
            "<mfrac linethickness=\"1em\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: Some(FontRelativeLength::from_ex(-1).into()),
                attr: None,
            }),
            "<mfrac linethickness=\"-1ex\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: Some(AbsoluteLength::from_pt(2).into()),
                attr: None,
            }),
            "<mfrac linethickness=\"2pt\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                // 8/20 = 4/10
                lt: Some(AbsoluteLength::from_twip(-8).into()),
                attr: None,
            }),
            "<mfrac linethickness=\"-0.4pt\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: None,
                attr: Some(FracAttr::DisplayStyleTrue),
            }),
            "<mfrac displaystyle=\"true\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: Some(AbsoluteLength::from_pt(0).into()),
                attr: Some(FracAttr::DisplayStyleTrue),
            }),
            "<mfrac linethickness=\"0\" displaystyle=\"true\"><mn>1</mn><mn>2</mn></mfrac>"
        );
        assert_eq!(
            render(&Node::Frac {
                num,
                den,
                lt: None,
                attr: Some(FracAttr::DisplayStyleFalse),
            }),
            "<mfrac displaystyle=\"false\"><mn>1</mn><mn>2</mn></mfrac>"
        );
    }

    #[test]
    fn render_row() {
        let nodes = &[
            &Node::SingleLetterIdent('x', false),
            &Node::Operator(symbol::EQUALS_SIGN.into(), None),
            &Node::Number("1"),
        ];

        assert_eq!(
            render(&Node::Row {
                nodes,
                attr: RowAttr::Style(Style::DisplayStyle)
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
                crate::attribute::Size::Scale1,
                symbol::LEFT_PARENTHESIS,
            )),
            "<mo maxsize=\"1.2em\" minsize=\"1.2em\">(</mo>"
        );
        assert_eq!(
            render(&Node::SizedParen(
                crate::attribute::Size::Scale3,
                symbol::SOLIDUS,
            )),
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
                align: crate::attribute::Align::Center,
                attr: None,
            }),
            "<mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable>"
        );
    }

    #[test]
    fn render_slashed() {
        assert_eq!(
            render(&Node::Slashed(&Node::SingleLetterIdent('x', false))),
            "<mi>x&#x0338;</mi>"
        );
    }

    #[test]
    fn render_multiscript() {
        assert_eq!(
            render(&Node::Multiscript {
                base: &Node::SingleLetterIdent('x', false),
                sub: &Node::Number("1"),
            }),
            "<mmultiscripts><mi>x</mi><mprescripts/><mn>1</mn><mrow></mrow></mmultiscripts>"
        );
    }

    #[test]
    fn render_text_transform() {
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::SingleLetterIdent('a', true),
            }),
            "<mi mathvariant=\"normal\">a</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::SingleLetterIdent('a', false),
            }),
            "<mi mathvariant=\"normal\">a</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::CollectedLetters("abc"),
            }),
            "<mi>abc</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Normal,
                content: &Node::MultiLetterIdent("abc"),
            }),
            "<mi>abc</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::SingleLetterIdent('a', true),
            }),
            "<mi>𝐚</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::SingleLetterIdent('a', false),
            }),
            "<mi>𝒂</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::CollectedLetters("abc"),
            }),
            "<mi>𝒂𝒃𝒄</mi>"
        );
        assert_eq!(
            render(&Node::TextTransform {
                tf: MathVariant::Transform(TextTransform::BoldItalic),
                content: &Node::MultiLetterIdent("abc"),
            }),
            "<mi>abc</mi>"
        );
    }
}
