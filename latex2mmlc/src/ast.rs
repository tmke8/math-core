use crate::attribute::{Accent, Align, DisplayStyle, LineThickness, MathVariant, Stretchy};
use crate::ops::Op;

/// AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Number(String),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op, Option<Stretchy>),
    OperatorWithSpacing {
        op: Op,
        left: Option<&'static str>,
        right: Option<&'static str>,
    },
    MultiLetterIdent(String, Option<MathVariant>),
    Space(&'static str),
    Subscript(Box<Node>, Box<Node>),
    Superscript(Box<Node>, Box<Node>),
    SubSup {
        target: Box<Node>,
        sub: Box<Node>,
        sup: Box<Node>,
    },
    OverOp(Op, Accent, Box<Node>),
    UnderOp(Op, Accent, Box<Node>),
    Overset {
        over: Box<Node>,
        target: Box<Node>,
    },
    Underset {
        under: Box<Node>,
        target: Box<Node>,
    },
    UnderOver {
        target: Box<Node>,
        under: Box<Node>,
        over: Box<Node>,
    },
    Sqrt(Box<Node>),
    Root(Box<Node>, Box<Node>),
    Frac(Box<Node>, Box<Node>, LineThickness, Option<DisplayStyle>),
    Row(Vec<Node>),
    PseudoRow(Vec<Node>),
    Fenced {
        open: Op,
        close: Op,
        content: Box<Node>,
    },
    SizedParen {
        size: &'static str,
        paren: Op,
    },
    Text(String),
    Table(Vec<Node>, Align),
    ColumnSeparator,
    RowSeparator,
    Slashed(Box<Node>),
}

const INDENT: &'static str = "    ";

macro_rules! push {
    ($buf:expr, $($s:expr),+ $(,)?) => {{
        /* use std::ops::AddAssign;
        let mut len = 0;
        $(len.add_assign(AsRef::<str>::as_ref(&$s).len());)+
        $buf.reserve(len); */
        $($buf.push_str($s.as_ref());)+
    }};
}

impl Node {
    pub fn render(&self) -> String {
        let mut buf = String::new();
        self.emit(&mut buf, 0);
        buf
    }

    pub fn emit(&self, s: &mut String, base_indent: usize) {
        // Compute the indent for the children of the node.
        let child_indent = base_indent.saturating_add(1);

        if !matches!(self, Node::PseudoRow(_)) {
            // Get the base indent out of the way.
            new_line(s, base_indent);
        }

        match self {
            Node::Number(number) => push!(s, "<mn>", number, "</mn>"),
            Node::SingleLetterIdent(letter, var) => {
                match var {
                    Some(var) => push!(s, "<mi", var, ">"),
                    None => push!(s, "<mi>"),
                };
                s.push(*letter);
                s.push_str("</mi>");
            }
            Node::Operator(op, stretchy) => {
                match stretchy {
                    Some(stretchy) => push!(s, "<mo", stretchy, ">"),
                    None => push!(s, "<mo>"),
                }
                push!(s, op, "</mo>");
            }
            Node::OperatorWithSpacing { op, left, right } => {
                match (left, right) {
                    (Some(left), Some(right)) => {
                        push!(s, "<mo lspace=\"", left, "em\" rspace=\"", right, "em\">",)
                    }
                    (Some(left), None) => {
                        push!(s, "<mo lspace=\"", left, "em\">")
                    }
                    (None, Some(right)) => {
                        push!(s, "<mo rspace=\"", right, "em\">")
                    }
                    (None, None) => s.push_str("<mo>"),
                }
                push!(s, op, "</mo>");
            }
            Node::MultiLetterIdent(letters, var) => {
                match var {
                    Some(var) => push!(s, "<mi", var, ">"),
                    None => s.push_str("<mi>"),
                }
                push!(s, letters, "</mi>");
            }
            Node::Space(space) => push!(s, "<mspace width=\"", space, "em\"/>"),
            Node::Subscript(base, sub) => {
                push!(s, "<msub>");
                base.emit(s, child_indent);
                sub.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</msub>")
            }
            Node::Superscript(base, sup) => {
                push!(s, "<msup>");
                base.emit(s, child_indent);
                sup.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</msup>")
            }
            Node::SubSup { target, sub, sup } => {
                push!(s, "<msubsup>");
                target.emit(s, child_indent);
                sub.emit(s, child_indent);
                sup.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</msubsup>")
            }
            Node::OverOp(op, acc, target) => {
                push!(s, "<mover>");
                target.emit(s, child_indent);
                new_line(s, child_indent);
                push!(s, "<mo accent=\"", acc, "\">", op, "</mo>");
                new_line(s, base_indent);
                push!(s, "</mover>")
            }
            Node::UnderOp(op, acc, target) => {
                push!(s, "<munder>");
                target.emit(s, child_indent);
                new_line(s, child_indent);
                push!(s, "<mo accent=\"", acc, "\">", op, "</mo>");
                new_line(s, base_indent);
                push!(s, "</munder>")
            }
            Node::Overset { over, target } => {
                push!(s, "<mover>");
                target.emit(s, child_indent);
                over.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</mover>")
            }
            Node::Underset { under, target } => {
                push!(s, "<munder>");
                target.emit(s, child_indent);
                under.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</munder>")
            }
            Node::UnderOver {
                target,
                under,
                over,
            } => {
                push!(s, "<munderover>");
                target.emit(s, child_indent);
                under.emit(s, child_indent);
                over.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</munderover>")
            }
            Node::Sqrt(content) => {
                push!(s, "<msqrt>");
                content.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</msqrt>")
            }
            Node::Root(degree, content) => {
                push!(s, "<mroot>");
                content.emit(s, child_indent);
                degree.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</mroot>")
            }
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    push!(s, "<mfrac", lt, style, ">");
                } else {
                    push!(s, "<mfrac", lt, ">");
                }
                num.emit(s, child_indent);
                denom.emit(s, child_indent);
                new_line(s, base_indent);
                push!(s, "</mfrac>")
            }
            Node::Row(vec) => {
                push!(s, "<mrow>");
                for node in vec.iter() {
                    node.emit(s, child_indent);
                }
                new_line(s, base_indent);
                push!(s, "</mrow>")
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    node.emit(s, base_indent);
                }
            }
            Node::Fenced {
                open,
                close,
                content,
            } => {
                push!(s, "<mrow>");
                new_line(s, child_indent);
                push!(s, "<mo stretchy=\"true\" form=\"prefix\">", open, "</mo>");
                content.emit(s, child_indent);
                new_line(s, child_indent);
                push!(s, "<mo stretchy=\"true\" form=\"postfix\">", close, "</mo>");
                new_line(s, base_indent);
                push!(s, "</mrow>")
            }
            Node::SizedParen { size, paren } => {
                push!(s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\">", paren, "</mo>");
            }
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => push!(s, "<mi", var, ">", x.to_string(), "&#x0338;</mi>"),
                    None => push!(s, "<mi>", x.to_string(), "&#x0338;</mi>"),
                },
                Node::Operator(x, _) => push!(s, "<mo>{}&#x0338;</mo>", x),
                n => n.emit(s, base_indent),
            },
            Node::Table(content, align) => {
                let child_indent2 = child_indent.saturating_add(1);
                let child_indent3 = child_indent2.saturating_add(1);
                let odd_col = match align {
                    Align::Center => "<mtd>",
                    Align::Left => "<mtd style=\"text-align: left; padding-right: 0\">",
                    Align::Alternating => "<mtd style=\"text-align: right; padding-right: 0\">",
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
                new_line(s, child_indent);
                push!(s, "<mtr>");
                new_line(s, child_indent2);
                push!(s, odd_col);
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            new_line(s, child_indent2);
                            push!(s, "</mtd>");
                            col += 1;
                            if j < total_len {
                                new_line(s, child_indent2);
                                push!(s, if col % 2 == 0 { even_col } else { odd_col });
                            }
                        }
                        Node::RowSeparator => {
                            new_line(s, child_indent2);
                            push!(s, "</mtd>");
                            new_line(s, child_indent);
                            push!(s, "</mtr>");
                            if j < total_len {
                                new_line(s, child_indent);
                                push!(s, "<mtr>");
                                new_line(s, child_indent2);
                                push!(s, odd_col);
                            }
                            col = 1;
                        }
                        node => {
                            node.emit(s, child_indent3);
                        }
                    }
                }
                new_line(s, child_indent2);
                push!(s, "</mtd>");
                new_line(s, child_indent);
                push!(s, "</mtr>");
                new_line(s, base_indent);
                push!(s, "</mtable>")
            }
            Node::Text(text) => push!(s, "<mtext>", text, "</mtext>"),
            Node::ColumnSeparator | Node::RowSeparator => (),
        }
    }
}

fn new_line(s: &mut String, indent_num: usize) {
    s.push('\n');
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
            (Node::Number("3.14".to_owned()), "\n<mn>3.14</mn>"),
            (Node::SingleLetterIdent('x', None), "\n<mi>x</mi>"),
            (Node::SingleLetterIdent('α', None), "\n<mi>α</mi>"),
            (
                Node::SingleLetterIdent('あ', Some(MathVariant::Normal)),
                "\n<mi mathvariant=\"normal\">あ</mi>",
            ),
        ];
        for (problem, answer) in problems.iter() {
            assert_eq!(&problem.render(), answer);
        }
    }
}
