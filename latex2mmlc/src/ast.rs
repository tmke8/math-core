use crate::attribute::{
    Accent, Align, DisplayStyle, LineThickness, MathVariant, PhantomWidth, Stretchy,
};
use crate::ops::Op;

/// AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Number(String),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op, Option<Stretchy>),
    OperatorWithSpacing {
        op: Op,
        stretchy: Option<Stretchy>,
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
    Phantom(Box<Node>, PhantomWidth),
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

const INDENT: &str = "    ";

macro_rules! push {
    ($buf:expr, $($s:expr),+ $(,)?) => {{
        /* use std::ops::AddAssign;
        let mut len = 0;
        $(len.add_assign(AsRef::<str>::as_ref(&$s).len());)+
        $buf.reserve(len); */
        $($buf.push_str($s.as_ref());)+
    }};
}

macro_rules! pushln {
    ($buf:expr, $indent:expr, $($s:expr),+ $(,)?) => {
        new_line_and_indent($buf, $indent);
        push!($buf, $($s),+)
    };
}

impl Node {
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

        // Buffer for `char::encode_utf8()`.
        // Not all branches use this, but it's just 4 bytes.
        let mut b = [0; 4];

        match self {
            Node::Number(number) => push!(s, "<mn>", number, "</mn>"),
            Node::SingleLetterIdent(letter, var) => {
                match var {
                    Some(var) => push!(s, "<mi", var, ">"),
                    None => push!(s, "<mi>"),
                };
                push!(s, letter.encode_utf8(&mut b), "</mi>");
            }
            Node::Operator(op, stretchy) => {
                match stretchy {
                    Some(stretchy) => push!(s, "<mo", stretchy, ">"),
                    None => push!(s, "<mo>"),
                }
                push!(s, op.str_ref(&mut b), "</mo>");
            }
            Node::OperatorWithSpacing { op, stretchy, left, right } => {
                match (left, right) {
                    (Some(left), Some(right)) => {
                        push!(s, "<mo lspace=\"", left, "em\" rspace=\"", right, "em\"",)
                    }
                    (Some(left), None) => {
                        push!(s, "<mo lspace=\"", left, "em\"")
                    }
                    (None, Some(right)) => {
                        push!(s, "<mo rspace=\"", right, "em\"")
                    }
                    (None, None) => s.push_str("<mo"),
                }
                if let Some(stretchy) = stretchy {
                    push!(s, stretchy);
                }
                push!(s, ">", op.str_ref(&mut b), "</mo>");
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
                pushln!(s, base_indent, "</msub>");
            }
            Node::Superscript(base, sup) => {
                push!(s, "<msup>");
                base.emit(s, child_indent);
                sup.emit(s, child_indent);
                pushln!(s, base_indent, "</msup>");
            }
            Node::SubSup { target, sub, sup } => {
                push!(s, "<msubsup>");
                target.emit(s, child_indent);
                sub.emit(s, child_indent);
                sup.emit(s, child_indent);
                pushln!(s, base_indent, "</msubsup>");
            }
            Node::OverOp(op, acc, target) => {
                let op = op.str_ref(&mut b);
                push!(s, "<mover>");
                target.emit(s, child_indent);
                pushln!(s, child_indent, "<mo accent=\"", acc, "\">", op, "</mo>");
                pushln!(s, base_indent, "</mover>");
            }
            Node::UnderOp(op, acc, target) => {
                push!(s, "<munder>");
                target.emit(s, child_indent);
                pushln!(s, child_indent, "<mo accent=\"", acc, "\">");
                push!(s, op.str_ref(&mut b), "</mo>");
                pushln!(s, base_indent, "</munder>");
            }
            Node::Overset { over, target } => {
                push!(s, "<mover>");
                target.emit(s, child_indent);
                over.emit(s, child_indent);
                pushln!(s, base_indent, "</mover>");
            }
            Node::Underset { under, target } => {
                push!(s, "<munder>");
                target.emit(s, child_indent);
                under.emit(s, child_indent);
                pushln!(s, base_indent, "</munder>");
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
                pushln!(s, base_indent, "</munderover>");
            }
            Node::Sqrt(content) => {
                push!(s, "<msqrt>");
                content.emit(s, child_indent);
                pushln!(s, base_indent, "</msqrt>");
            }
            Node::Root(degree, content) => {
                push!(s, "<mroot>");
                content.emit(s, child_indent);
                degree.emit(s, child_indent);
                pushln!(s, base_indent, "</mroot>");
            }
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    push!(s, "<mfrac", lt, style, ">");
                } else {
                    push!(s, "<mfrac", lt, ">");
                }
                num.emit(s, child_indent);
                denom.emit(s, child_indent);
                pushln!(s, base_indent, "</mfrac>");
            }
            Node::Row(vec) => {
                push!(s, "<mrow>");
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
            } => {
                push!(s, "<mrow>");
                pushln!(s, child_indent, "<mo stretchy=\"true\" form=\"prefix\">");
                push!(s, open.str_ref(&mut b), "</mo>");
                content.emit(s, child_indent);
                pushln!(s, child_indent, "<mo stretchy=\"true\" form=\"postfix\">");
                push!(s, close.str_ref(&mut b), "</mo>");
                pushln!(s, base_indent, "</mrow>");
            }
            Node::SizedParen { size, paren } => {
                push!(s, "<mo maxsize=\"", size, "\" minsize=\"", size, "\">");
                push!(s, paren.str_ref(&mut b), "</mo>");
            }
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => {
                        push!(s, "<mi", var, ">", x.encode_utf8(&mut b), "&#x0338;</mi>")
                    }
                    None => push!(s, "<mi>", x.encode_utf8(&mut b), "&#x0338;</mi>"),
                },
                Node::Operator(x, _) => {
                    push!(s, "<mo>{}&#x0338;</mo>", x.str_ref(&mut b))
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
            Node::Text(text) => push!(s, "<mtext>", text, "</mtext>"),
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
            (Node::Number("3.14".to_owned()), "<mn>3.14</mn>"),
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
