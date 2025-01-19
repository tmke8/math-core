#[cfg(test)]
use serde::Serialize;

use crate::arena::NodeList;
use crate::attribute::{Accent, Align, FracAttr, MathSpacing, MathVariant, OpAttr, Style};
use crate::ops::Op;

/// AST node
#[derive(Debug)]
#[cfg_attr(test, derive(Serialize))]
pub enum Node<'arena> {
    Number(&'arena str),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op, Option<OpAttr>),
    OpGreaterThan,
    OpLessThan,
    OpAmpersand,
    OperatorWithSpacing {
        op: Op,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    MultiLetterIdent(&'arena str),
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
    OverOp(Op, Accent, Option<OpAttr>, &'arena Node<'arena>),
    UnderOp(Op, Accent, &'arena Node<'arena>),
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
        lt: Option<char>,
        attr: Option<FracAttr>,
    },
    Row {
        nodes: NodeList<'arena>,
        style: Option<Style>,
    },
    PseudoRow(NodeList<'arena>),
    Mathstrut,
    Fenced {
        open: Op,
        close: Op,
        style: Option<Style>,
        stretchy: bool,
        content: &'arena Node<'arena>,
    },
    SizedParen {
        size: &'static str,
        paren: Op,
        stretchy: bool,
    },
    Text(&'arena str),
    Table {
        content: NodeList<'arena>,
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

impl<'arena> Node<'arena> {
    pub fn render(&'arena self) -> String {
        let mut buf = String::new();
        let mut emitter = MathMLEmitter::new(&mut buf);
        emitter.emit(self, 0);
        buf
    }
}

pub struct MathMLEmitter<'output> {
    s: &'output mut String,
}

impl<'output> MathMLEmitter<'output> {
    pub fn new(buf: &'output mut String) -> Self {
        Self { s: buf }
    }

    pub fn emit(&mut self, node: &Node<'_>, base_indent: usize) {
        // Compute the indent for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        if !matches!(
            node,
            Node::PseudoRow(_) | Node::ColumnSeparator | Node::RowSeparator
        ) {
            // Get the base indent out of the way.
            new_line_and_indent(&mut self.s, base_indent);
        }

        match node {
            Node::Number(number) => push!(self.s, "<mn>", number, "</mn>"),
            Node::SingleLetterIdent(letter, var) => {
                match var {
                    Some(var) => push!(self.s, "<mi", var, ">"),
                    None => push!(self.s, "<mi>"),
                };
                push!(self.s, @*letter, "</mi>");
            }
            Node::Operator(op, attributes) => {
                match attributes {
                    Some(attributes) => push!(self.s, "<mo", attributes, ">"),
                    None => push!(self.s, "<mo>"),
                }
                push!(self.s, @op, "</mo>");
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
                        push!(self.s, "<mo lspace=\"", left, "\" rspace=\"", right, "\"")
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
                pushln!(self.s, base_indent, close);
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
                pushln!(self.s, base_indent, close);
            }
            Node::Multiscript { base, sub } => {
                push!(self.s, "<mmultiscripts>");
                self.emit(base, child_indent);
                pushln!(self.s, child_indent, "<mprescripts/>");
                self.emit(sub, child_indent);
                pushln!(self.s, child_indent, "<mrow></mrow>");
                pushln!(self.s, base_indent, "</mmultiscripts>");
            }
            Node::OverOp(op, acc, attr, target) => {
                push!(self.s, "<mover>");
                self.emit(target, child_indent);
                pushln!(self.s, child_indent, "<mo accent=\"", acc, "\"");
                if let Some(attr) = attr {
                    push!(self.s, attr);
                }
                push!(self.s, ">", @op, "</mo>");
                pushln!(self.s, base_indent, "</mover>");
            }
            Node::UnderOp(op, acc, target) => {
                push!(self.s, "<munder>");
                self.emit(target, child_indent);
                pushln!(self.s, child_indent, "<mo accent=\"", acc, "\">", @op, "</mo>");
                pushln!(self.s, base_indent, "</munder>");
            }
            Node::Sqrt(content) => {
                push!(self.s, "<msqrt>");
                self.emit(content, child_indent);
                pushln!(self.s, base_indent, "</msqrt>");
            }
            Node::Frac { num, den, lt, attr } => {
                push!(self.s, "<mfrac");
                if let Some(lt) = lt {
                    push!(self.s, " linethickness=\"", @*lt, "pt\"");
                }
                if let Some(style) = attr {
                    push!(self.s, style);
                }
                push!(self.s, ">");
                self.emit(num, child_indent);
                self.emit(den, child_indent);
                pushln!(self.s, base_indent, "</mfrac>");
            }
            Node::Row { nodes, style } => {
                match style {
                    Some(style) => push!(self.s, "<mrow", style, ">"),
                    None => push!(self.s, "<mrow>"),
                }
                for node in nodes.iter() {
                    self.emit(node, child_indent);
                }
                pushln!(self.s, base_indent, "</mrow>");
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    self.emit(node, base_indent);
                }
            }
            Node::Mathstrut => {
                push!(
                    self.s,
                    r#"<mpadded width="0" style="visibility:hidden"><mo stretchy="false">(</mo></mpadded>"#
                );
            }
            Node::Fenced {
                open,
                close,
                style,
                stretchy,
                content,
            } => {
                match style {
                    Some(style) => push!(self.s, "<mrow", style, ">"),
                    None => push!(self.s, "<mrow>"),
                }
                pushln!(self.s, child_indent, "<mo");
                if *stretchy {
                    // TODO: Should we set `symmetric="true"` as well?
                    push!(self.s, " stretchy=\"true\"");
                }
                push!(self.s, ">");
                if char::from(open) != '\0' {
                    push!(self.s, @open);
                }
                push!(self.s, "</mo>");
                self.emit(content, child_indent);
                pushln!(self.s, child_indent, "<mo");
                if *stretchy {
                    // TODO: Should we set `symmetric="true"` as well?
                    push!(self.s, " stretchy=\"true\"");
                }
                push!(self.s, ">");
                if char::from(close) != '\0' {
                    push!(self.s, @close);
                }
                push!(self.s, "</mo>");
                pushln!(self.s, base_indent, "</mrow>");
            }
            Node::SizedParen {
                size,
                paren,
                stretchy,
            } => {
                push!(self.s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\"");
                if *stretchy {
                    push!(self.s, " stretchy=\"true\" symmetric=\"true\"");
                }
                push!(self.s, ">", @paren, "</mo>");
            }
            Node::Slashed(node) => match node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => {
                        push!(self.s, "<mi", var, ">", @*x, "&#x0338;</mi>")
                    }
                    None => push!(self.s, "<mi>", @*x, "&#x0338;</mi>"),
                },
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
                    Align::Alternating => "<mtd style=\"text-align: -webkit-left; text-align: -moz-left; padding-left: 0\">",
                };

                let mut col: usize = 1;
                push!(self.s, "<mtable");
                if let Some(attr) = attr {
                    push!(self.s, attr);
                }
                push!(self.s, ">");
                pushln!(self.s, child_indent, "<mtr>");
                pushln!(self.s, child_indent2, odd_col);
                for node in content.iter() {
                    match node {
                        Node::ColumnSeparator => {
                            pushln!(self.s, child_indent2, "</mtd>");
                            col += 1;
                            pushln!(
                                &mut self.s,
                                child_indent2,
                                if col % 2 == 0 { even_col } else { odd_col }
                            );
                        }
                        Node::RowSeparator => {
                            pushln!(self.s, child_indent2, "</mtd>");
                            pushln!(self.s, child_indent, "</mtr>");
                            pushln!(self.s, child_indent, "<mtr>");
                            pushln!(self.s, child_indent2, odd_col);
                            col = 1;
                        }
                        node => {
                            self.emit(node, child_indent3);
                        }
                    }
                }
                pushln!(self.s, child_indent2, "</mtd>");
                pushln!(self.s, child_indent, "</mtr>");
                pushln!(self.s, base_indent, "</mtable>");
            }
            Node::Text(text) => {
                push!(self.s, "<mtext>", text, "</mtext>");
            }
            Node::ColumnSeparator | Node::RowSeparator => (),
        }
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
    use super::super::attribute::MathVariant;
    use super::Node;

    #[test]
    fn node_display() {
        let problems = vec![
            (Node::Number("3.14"), "<mn>3.14</mn>"),
            (Node::SingleLetterIdent('x', None), "<mi>x</mi>"),
            (Node::SingleLetterIdent('α', None), "<mi>α</mi>"),
            (
                Node::SingleLetterIdent('あ', Some(MathVariant::Normal)),
                "<mi mathvariant=\"normal\">あ</mi>",
            ),
        ];
        for (problem, answer) in problems.iter() {
            assert_eq!(&problem.render(), answer);
        }
    }
}
