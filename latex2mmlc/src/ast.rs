use crate::attribute::{
    Accent, Align, DisplayStyle, MathSpacing, MathVariant, OpAttr, PhantomWidth, Style,
};
use crate::ops::Op;

/// AST node
#[derive(Debug)]
pub enum Node<'a> {
    Number(&'a str),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op, Option<OpAttr>),
    OpGreaterThan,
    OpLessThan,
    OpAmpersand,
    OperatorWithSpacing {
        op: Op,
        attrs: Option<OpAttr>,
        left: Option<MathSpacing>,
        right: Option<MathSpacing>,
    },
    MultiLetterIdent(String),
    Space(&'static str),
    Subscript(Box<Node<'a>>, Box<Node<'a>>),
    Superscript(Box<Node<'a>>, Box<Node<'a>>),
    SubSup {
        target: Box<Node<'a>>,
        sub: Box<Node<'a>>,
        sup: Box<Node<'a>>,
    },
    OverOp(Op, Accent, Box<Node<'a>>),
    UnderOp(Op, Accent, Box<Node<'a>>),
    Overset {
        symbol: Box<Node<'a>>,
        target: Box<Node<'a>>,
    },
    Underset {
        symbol: Box<Node<'a>>,
        target: Box<Node<'a>>,
    },
    UnderOver {
        target: Box<Node<'a>>,
        under: Box<Node<'a>>,
        over: Box<Node<'a>>,
    },
    Sqrt(Box<Node<'a>>),
    Root(Box<Node<'a>>, Box<Node<'a>>),
    Frac(
        Box<Node<'a>>,
        Box<Node<'a>>,
        Option<char>,
        Option<DisplayStyle>,
    ),
    Row(Vec<Node<'a>>, Option<Style>),
    PseudoRow(Vec<Node<'a>>),
    Phantom(Box<Node<'a>>, PhantomWidth),
    Fenced {
        open: Op,
        close: Op,
        content: Box<Node<'a>>,
        style: Option<Style>,
    },
    SizedParen {
        size: &'static str,
        paren: Op,
    },
    Text(&'a str),
    Table(Vec<Node<'a>>, Align),
    ColumnSeparator,
    RowSeparator,
    Slashed(Box<Node<'a>>),
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

impl<'a> Node<'a> {
    pub fn render(&self) -> String {
        let mut buf = String::new();
        self.emit(&mut buf, 0);
        buf
    }

    pub fn emit(&self, s: &mut String, base_indent: usize) {
        // Compute the indent for the children of the node.
        let child_indent = if base_indent > 0 {
            base_indent.saturating_add(1)
        } else {
            0
        };

        if !matches!(self, Node::PseudoRow(_)) {
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
            Node::OpGreaterThan => push!(s, "<mo>&gt;</mo>"),
            Node::OpLessThan => push!(s, "<mo>&lt;</mo>"),
            Node::OpAmpersand => push!(s, "<mo>&amp;</mo>"),
            Node::OperatorWithSpacing {
                op,
                attrs,
                left,
                right,
            } => {
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
                if let Some(stretchy) = attrs {
                    push!(s, stretchy);
                }
                push!(s, ">", @op, "</mo>");
            }
            Node::MultiLetterIdent(letters) => push!(s, "<mi>", letters, "</mi>"),
            Node::Space(space) => push!(s, "<mspace width=\"", space, "em\"/>"),
            // The following nodes have exactly two children.
            node @ (Node::Subscript(first, second)
            | Node::Superscript(first, second)
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
                    Node::Subscript(_, _) => ("<msub>", "</msub>"),
                    Node::Superscript(_, _) => ("<msup>", "</msup>"),
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
            Node::Frac(num, denom, lt, style) => {
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
            Node::Row(vec, style) => {
                match style {
                    Some(style) => push!(s, "<mrow", style, ">"),
                    None => push!(s, "<mrow>"),
                }
                for node in vec.iter() {
                    node.emit(s, child_indent);
                }
                pushln!(s, base_indent, "</mrow>");
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    node.emit(s, base_indent);
                }
            }
            Node::Phantom(node, width) => {
                push!(s, "<mphantom", width, ">");
                node.emit(s, child_indent);
                pushln!(s, base_indent, "</mphantom>");
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
                push!(s, @open, "</mo>");
                content.emit(s, child_indent);
                pushln!(s, child_indent, "<mo stretchy=\"true\" form=\"postfix\">");
                push!(s, @close, "</mo>");
                pushln!(s, base_indent, "</mrow>");
            }
            Node::SizedParen { size, paren } => {
                push!(s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\">");
                push!(s, @paren, "</mo>");
            }
            Node::Slashed(node) => match &**node {
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
            Node::Table(content, align) => {
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
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            pushln!(s, child_indent2, "</mtd>");
                            col += 1;
                            if j < total_len {
                                pushln!(
                                    s,
                                    child_indent2,
                                    if col % 2 == 0 { even_col } else { odd_col }
                                );
                            }
                        }
                        Node::RowSeparator => {
                            pushln!(s, child_indent2, "</mtd>");
                            pushln!(s, child_indent, "</mtr>");
                            if j < total_len {
                                pushln!(s, child_indent, "<mtr>");
                                pushln!(s, child_indent2, odd_col);
                            }
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
                push!(s, "<mtext>");
                // Replace spaces with non-breaking spaces.
                s.extend(text.chars().map(|c| if c == ' ' { '\u{A0}' } else { c }));
                push!(s, "</mtext>");
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
