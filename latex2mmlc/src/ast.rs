use crate::ops::Op;

use super::attribute::{Accent, Align, DisplayStyle, LineThickness, MathVariant};
use std::fmt;

/// AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Number(String),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op),
    OperatorWithSpacing {
        op: Op,
        left: f32,
        right: f32,
    },
    MultiLetterIdent(String, Option<MathVariant>),
    Space(f32),
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
    StretchedOp(bool, Op),
    Paren(Op),
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

impl Node {
    pub fn render(&self, f: &mut fmt::Formatter<'_>, base_indent: usize) -> fmt::Result {
        let emit = Emitter {
            indent_num: base_indent,
        };

        // Compute the indent for the children of the node.
        let child_indent = base_indent.saturating_add(1);

        if !matches!(self, Node::PseudoRow(_)) {
            // Get the base indent out of the way.
            new_line(f, base_indent)?;
        }

        match self {
            Node::Number(number) => write!(f, "<mn>{}</mn>", number),
            Node::SingleLetterIdent(letter, var) => match var {
                Some(var) => write!(f, "<mi{}>{}</mi>", var, letter),
                None => write!(f, "<mi>{}</mi>", letter),
            },
            Node::Operator(op) => write!(f, "<mo>{}</mo>", op.char()),
            Node::OperatorWithSpacing { op, left, right } => {
                match (left.is_finite(), right.is_finite()) {
                    (true, true) => write!(
                        f,
                        r#"<mo lspace="{}em" rspace="{}em">{}</mo>"#,
                        left,
                        right,
                        op.char()
                    ),
                    (true, false) => {
                        write!(f, r#"<mo lspace="{}em">{}</mo>"#, left, op.char())
                    }
                    (false, true) => {
                        write!(f, r#"<mo rspace="{}em">{}</mo>"#, right, op.char())
                    }
                    (false, false) => write!(f, "<mo>{}</mo>", op.char()),
                }
            }
            Node::MultiLetterIdent(letters, var) => match var {
                Some(var) => write!(f, "<mi{}>{}</mi>", var, letters),
                None => write!(f, "<mi>{}</mi>", letters),
            },
            Node::Space(space) => write!(f, r#"<mspace width="{}em"/>"#, space),
            Node::Subscript(base, sub) => {
                write!(f, "<msub>")?;
                base.render(f, child_indent)?;
                sub.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</msub>")
            }
            Node::Superscript(base, sup) => {
                write!(f, "<msup>")?;
                base.render(f, child_indent)?;
                sup.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</msup>")
            }
            Node::SubSup { target, sub, sup } => {
                write!(f, "<msubsup>")?;
                target.render(f, child_indent)?;
                sub.render(f, child_indent)?;
                sup.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</msubsup>")
            }
            Node::OverOp(op, acc, target) => {
                write!(f, "<mover>")?;
                target.render(f, child_indent)?;
                new_line(f, child_indent)?;
                write!(f, r#"<mo accent="{}">{}</mo>"#, acc, op.char())?;
                new_line(f, base_indent)?;
                write!(f, "</mover>")
            }
            Node::UnderOp(op, acc, target) => {
                write!(f, "<munder>")?;
                target.render(f, child_indent)?;
                new_line(f, child_indent)?;
                write!(f, r#"<mo accent="{}">{}</mo>"#, acc, op.char())?;
                new_line(f, base_indent)?;
                write!(f, "</munder>")
            }
            Node::Overset { over, target } => {
                write!(f, "<mover>")?;
                target.render(f, child_indent)?;
                over.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</mover>")
            }
            Node::Underset { under, target } => {
                write!(f, "<munder>")?;
                target.render(f, child_indent)?;
                under.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</munder>")
            }
            Node::UnderOver {
                target,
                under,
                over,
            } => {
                write!(f, "<munderover>")?;
                target.render(f, child_indent)?;
                under.render(f, child_indent)?;
                over.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</munderover>")
            }
            Node::Sqrt(content) => {
                write!(f, "<msqrt>")?;
                content.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</msqrt>")
            }
            Node::Root(degree, content) => {
                write!(f, "<mroot>")?;
                content.render(f, child_indent)?;
                degree.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</mroot>")
            }
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    write!(f, "<mfrac{}{}>", lt, style)?;
                } else {
                    write!(f, "<mfrac{}>", lt)?;
                }
                num.render(f, child_indent)?;
                denom.render(f, child_indent)?;
                new_line(f, base_indent)?;
                write!(f, "</mfrac>")
            }
            Node::Row(vec) => {
                write!(f, "<mrow>")?;
                for node in vec.iter() {
                    node.render(f, child_indent)?;
                }
                new_line(f, base_indent)?;
                write!(f, "</mrow>")
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    node.render(f, base_indent)?;
                }
                Ok(())
            }
            Node::Fenced {
                open,
                close,
                content,
            } => {
                write!(f, "<mrow>")?;
                new_line(f, child_indent)?;
                write!(
                    f,
                    "<mo stretchy=\"true\" form=\"prefix\">{}</mo>",
                    open.char()
                )?;
                content.render(f, child_indent)?;
                new_line(f, child_indent)?;
                write!(
                    f,
                    "<mo stretchy=\"true\" form=\"postfix\">{}</mo>",
                    close.char(),
                )?;
                new_line(f, base_indent)?;
                write!(f, "</mrow>")
            }
            Node::StretchedOp(stretchy, op) => {
                write!(f, r#"<mo stretchy="{}">{}</mo>"#, stretchy, op.char())
            }
            Node::Paren(op) => write!(f, r#"<mo stretchy="false">{}</mo>"#, op.char()),
            Node::SizedParen { size, paren } => write!(
                f,
                r#"<mrow><mo maxsize="{0}" minsize="{0}">{1}</mro></mrow>"#,
                size,
                paren.char()
            ),
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => write!(f, "<mi{}>{}&#x0338;</mi>", var, x),
                    None => write!(f, "<mi>{}&#x0338;</mi>", x),
                },
                Node::Operator(x) => write!(f, "<mo>{}&#x0338;</mo>", x.char()),
                n => n.render(f, base_indent),
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
                write!(f, "<mtable>")?;
                new_line(f, child_indent)?;
                write!(f, "<mtr>")?;
                new_line(f, child_indent2)?;
                write!(f, "{}", odd_col)?;
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            new_line(f, child_indent2)?;
                            write!(f, "</mtd>")?;
                            col += 1;
                            if j < total_len {
                                new_line(f, child_indent2)?;
                                write!(f, "{}", if col % 2 == 0 { even_col } else { odd_col },)?;
                            }
                        }
                        Node::RowSeparator => {
                            new_line(f, child_indent2)?;
                            write!(f, "</mtd>")?;
                            new_line(f, child_indent)?;
                            write!(f, "</mtr>")?;
                            if j < total_len {
                                new_line(f, child_indent)?;
                                write!(f, "<mtr>")?;
                                new_line(f, child_indent2)?;
                                write!(f, "{}", odd_col)?;
                            }
                            col = 1;
                        }
                        node => {
                            node.render(f, child_indent3)?;
                        }
                    }
                }
                new_line(f, child_indent2)?;
                write!(f, "</mtd>")?;
                new_line(f, child_indent)?;
                write!(f, "</mtr>")?;
                new_line(f, base_indent)?;
                write!(f, "</mtable>")
            }
            Node::Text(text) => write!(f, "<mtext>{}</mtext>", text),
            Node::ColumnSeparator | Node::RowSeparator => Ok(()),
        }
    }
}

fn new_line(f: &mut fmt::Formatter<'_>, indent_num: usize) -> fmt::Result {
    write!(f, "\n")?;
    for _ in 0..indent_num {
        write!(f, "{}", INDENT)?;
    }
    Ok(())
}

struct Emitter {
    indent_num: usize,
}

impl Emitter {
    fn line(self: &Emitter, f: &mut fmt::Formatter<'_>, content: &str) -> fmt::Result {
        new_line(f, self.indent_num)?;
        write!(f, "{}", content)?;
        Ok(())
    }

    fn indented_line(
        self: &Emitter,
        f: &mut fmt::Formatter<'_>,
        content: &str,
        additional_indent: usize,
    ) -> fmt::Result {
        new_line(f, self.indent_num.saturating_add(additional_indent))?;
        write!(f, "{}", content)?;
        Ok(())
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render(f, 0)
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
            assert_eq!(&format!("{}", problem), answer);
        }
    }
}
