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
    OverOp(Op, Accent, &'arena Node<'arena>),
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
        num: &'arena Node<'arena>,
        denom: &'arena Node<'arena>,
        lt: Option<char>,
        style: Option<FracAttr>,
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
        content: &'arena Node<'arena>,
        style: Option<Style>,
    },
    SizedParen {
        size: &'static str,
        paren: Op,
    },
    Text(&'arena str),
    Table {
        content: NodeList<'arena>,
        align: Align,
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
        self.emit(&mut buf, 0);
        buf
    }

    pub fn emit(&'arena self, s: &mut String, base_indent: usize) {
        // Compute the indent for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        if !matches!(
            self,
            Node::PseudoRow(_) | Node::ColumnSeparator | Node::RowSeparator
        ) {
            // Get the base indent out of the way.
            new_line_and_indent(s, base_indent);
        }

        match self {
            Node::Number(number) => push!(s, "<mn>", number, "</mn>"),
            Node::SingleLetterIdent(letter, var) => {
                match var {
                    Some(var) => push!(s, "<mi", var, ">"),
                    None => push!(s, "<mi>"),
                };
                push!(s, @*letter, "</mi>");
            }
            Node::Operator(op, attributes) => {
                match attributes {
                    Some(attributes) => push!(s, "<mo", attributes, ">"),
                    None => push!(s, "<mo>"),
                }
                push!(s, @op, "</mo>");
            }
            node @ (Node::OpGreaterThan | Node::OpLessThan | Node::OpAmpersand) => {
                let op = match node {
                    Node::OpGreaterThan => "&gt;",
                    Node::OpLessThan => "&lt;",
                    Node::OpAmpersand => "&amp;",
                    _ => unreachable!(),
                };
                push!(s, "<mo>", op, "</mo>");
            }
            Node::OperatorWithSpacing { op, left, right } => {
                match (left, right) {
                    (Some(left), Some(right)) => {
                        push!(s, "<mo lspace=\"", left, "\" rspace=\"", right, "\"",)
                    }
                    (Some(left), None) => {
                        push!(s, "<mo lspace=\"", left, "\"")
                    }
                    (None, Some(right)) => {
                        push!(s, "<mo rspace=\"", right, "\"")
                    }
                    (None, None) => s.push_str("<mo"),
                }
                push!(s, ">", @op, "</mo>");
            }
            Node::MultiLetterIdent(letters) => {
                push!(s, "<mi>", letters, "</mi>");
            }
            Node::Space(space) => push!(s, "<mspace width=\"", space, "em\"/>"),
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
                push!(s, open);
                first.emit(s, child_indent);
                second.emit(s, child_indent);
                pushln!(s, base_indent, close);
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
                push!(s, open);
                first.emit(s, child_indent);
                second.emit(s, child_indent);
                third.emit(s, child_indent);
                pushln!(s, base_indent, close);
            }
            Node::Multiscript { base, sub } => {
                push!(s, "<mmultiscripts>");
                base.emit(s, child_indent);
                pushln!(s, child_indent, "<mprescripts/>");
                sub.emit(s, child_indent);
                pushln!(s, child_indent, "<mrow></mrow>");
                pushln!(s, base_indent, "</mmultiscripts>");
            }
            Node::OverOp(op, acc, target) => {
                push!(s, "<mover>");
                target.emit(s, child_indent);
                pushln!(s, child_indent, "<mo accent=\"", acc, "\">", @op, "</mo>");
                pushln!(s, base_indent, "</mover>");
            }
            Node::UnderOp(op, acc, target) => {
                push!(s, "<munder>");
                target.emit(s, child_indent);
                pushln!(s, child_indent, "<mo accent=\"", acc, "\">", @op, "</mo>");
                pushln!(s, base_indent, "</munder>");
            }
            Node::Sqrt(content) => {
                push!(s, "<msqrt>");
                content.emit(s, child_indent);
                pushln!(s, base_indent, "</msqrt>");
            }
            Node::Frac {
                num,
                denom,
                lt,
                style,
            } => {
                push!(s, "<mfrac");
                if let Some(lt) = lt {
                    push!(s, " linethickness=\"", @*lt, "pt\"");
                }
                if let Some(style) = style {
                    push!(s, style);
                }
                push!(s, ">");
                num.emit(s, child_indent);
                denom.emit(s, child_indent);
                pushln!(s, base_indent, "</mfrac>");
            }
            Node::Row { nodes, style } => {
                match style {
                    Some(style) => push!(s, "<mrow", style, ">"),
                    None => push!(s, "<mrow>"),
                }
                for node in nodes.iter() {
                    node.emit(s, child_indent);
                }
                pushln!(s, base_indent, "</mrow>");
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    node.emit(s, base_indent);
                }
            }
            Node::Mathstrut => {
                push!(
                    s,
                    r#"<mpadded width="0" style="visibility:hidden"><mo stretchy="false">(</mo></mpadded>"#
                );
            }
            Node::Fenced {
                open,
                close,
                content,
                style,
            } => {
                match style {
                    Some(style) => push!(s, "<mrow", style, ">"),
                    None => push!(s, "<mrow>"),
                }
                pushln!(s, child_indent, "<mo stretchy=\"true\" form=\"prefix\">");
                if char::from(open) != '\0' {
                    push!(s, @open);
                }
                push!(s, "</mo>");
                content.emit(s, child_indent);
                pushln!(s, child_indent, "<mo stretchy=\"true\" form=\"postfix\">");
                if char::from(close) != '\0' {
                    push!(s, @close);
                }
                push!(s, "</mo>");
                pushln!(s, base_indent, "</mrow>");
            }
            Node::SizedParen { size, paren } => {
                push!(s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\">");
                push!(s, @paren, "</mo>");
            }
            Node::Slashed(node) => match node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => {
                        push!(s, "<mi", var, ">", @*x, "&#x0338;</mi>")
                    }
                    None => push!(s, "<mi>", @*x, "&#x0338;</mi>"),
                },
                Node::Operator(x, _) => {
                    push!(s, "<mo>", @x, "&#x0338;</mo>");
                }
                n => n.emit(s, base_indent),
            },
            Node::Table { content, align } => {
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
                    Align::Left => r#"<mtd style="text-align: left; padding-right: 0">"#,
                    Align::Alternating => r#"<mtd style="text-align: right; padding-right: 0">"#,
                };
                let even_col = match align {
                    Align::Center => "<mtd>",
                    Align::Left => {
                        "<mtd style=\"text-align: left; padding-right: 0; padding-left: 1em\">"
                    }
                    Align::Alternating => "<mtd style=\"text-align: left; padding-left: 0\">",
                };

                let mut col: usize = 1;
                push!(s, "<mtable>");
                pushln!(s, child_indent, "<mtr>");
                pushln!(s, child_indent2, odd_col);
                for node in content.iter() {
                    match node {
                        Node::ColumnSeparator => {
                            pushln!(s, child_indent2, "</mtd>");
                            col += 1;
                            pushln!(
                                s,
                                child_indent2,
                                if col % 2 == 0 { even_col } else { odd_col }
                            );
                        }
                        Node::RowSeparator => {
                            pushln!(s, child_indent2, "</mtd>");
                            pushln!(s, child_indent, "</mtr>");
                            pushln!(s, child_indent, "<mtr>");
                            pushln!(s, child_indent2, odd_col);
                            col = 1;
                        }
                        node => {
                            node.emit(s, child_indent3);
                        }
                    }
                }
                pushln!(s, child_indent2, "</mtd>");
                pushln!(s, child_indent, "</mtr>");
                pushln!(s, base_indent, "</mtable>");
            }
            Node::Text(text) => {
                push!(s, "<mtext>", text, "</mtext>");
            }
            Node::ColumnSeparator | Node::RowSeparator => (),
        }
    }
}

// impl fmt::Display for Node<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Node::Number(num) => writeln!(f, "Number({}),", num),
//             Node::SingleLetterIdent(x, var) => match var {
//                 Some(var) => writeln!(f, "SingleLetterIdent({}, Some({:?})),", x, var),
//                 None => writeln!(f, "SingleLetterIdent({}, None),", x),
//             },
//             Node::Operator(x, var) => match var {
//                 Some(var) => writeln!(f, "Operator({:?}, Some({:?})),", x, var),
//                 None => writeln!(f, "Operator({}, None),", x),
//             },
//             Node::OpGreaterThan => writeln!(f, "OpGreaterThan,"),
//             Node::OpLessThan => writeln!(f, "OpLessThan,"),
//             Node::OpAmpersand => writeln!(f, "OpAmpersand,"),
//             Node::OperatorWithSpacing { op, left, right } => {
//                 writeln!(
//                     f,
//                     "OperatorWithSpacing{{ op: {}, left: {:?}, right: {:?} }},",
//                     op, left, right
//                 )
//             }
//             Node::MultiLetterIdent(ident) => writeln!(f, "MultiLetterIdent({}),", ident),
//             Node::Space(space) => writeln!(f, "Space({}),", space),
//             Node::Subscript { target, symbol } => {
//                 writeln!(
//                     f,
//                     "Subscript {{\n    target: {},\n    symbol: {}\n}},",
//                     target, symbol
//                 )
//             }
//             Node::Superscript { target, symbol } => {
//                 writeln!(
//                     f,
//                     "Superscript {{\n    target: {},\n    symbol: {}\n}},",
//                     target, symbol
//                 )
//             }
//             Node::SubSup { target, sub, sup } => {
//                 writeln!(
//                     f,
//                     "SubSup {{\n    target: {},\n    sub: {},\n    sup: {}\n}},",
//                     target, sub, sup
//                 )
//             }
//             Node::OverOp(op, accent, target) => {
//                 writeln!(
//                     f,
//                     "OverOp {{\n    op: {},\n    accent: {:?},\n    target: {}\n}},",
//                     op, accent, target
//                 )
//             }
//             Node::UnderOp(op, accent, target) => {
//                 writeln!(
//                     f,
//                     "UnderOp {{\n    op: {},\n    accent: {:?},\n    target: {}\n}},",
//                     op, accent, target
//                 )
//             }
//             Node::Overset { symbol, target } => {
//                 writeln!(
//                     f,
//                     "Overset {{\n    symbol: {},\n    target: {}\n}},",
//                     symbol, target
//                 )
//             }
//             Node::Underset { symbol, target } => {
//                 writeln!(
//                     f,
//                     "Underset {{\n    symbol: {},\n    target: {}\n}},",
//                     symbol, target
//                 )
//             }
//             Node::UnderOver {
//                 target,
//                 under,
//                 over,
//             } => {
//                 writeln!(
//                     f,
//                     "UnderOver {{\n    target: {},\n    under: {},\n    over: {}\n}},",
//                     target, under, over
//                 )
//             }
//             Node::Sqrt(target) => writeln!(f, "Sqrt(\n    {}\n),", target),
//             Node::Root(target, nth) => {
//                 writeln!(f, "Root {{\n    target: {},\n    nth: {}\n}},", target, nth)
//             }
//             Node::Frac(num, denom, lt, style) => {
//                 writeln!(
//                     f,
//                     "Frac {{\n    num: {},\n    denom: {},\n    lt: {:?}, style: {:?}}},",
//                     num, denom, lt, style
//                 )
//             }
//             Node::Row(nodes, style) => {
//                 writeln!(f, "Row {{\n    nodes: {:?},\n    style: {:?}\n}},", nodes, style)
//             }
//         }
//     }
// }

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
