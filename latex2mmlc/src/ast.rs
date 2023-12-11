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
        // Compute the indent for the children of the node.
        let child_indent = base_indent.saturating_add(1);

        if !matches!(self, Node::PseudoRow(_)) {
            // Get the base indent out of the way.
            write_indent(f, base_indent)?;
        }

        match self {
            Node::Number(number) => writeln!(f, "<mn>{}</mn>", number),
            Node::SingleLetterIdent(letter, var) => match var {
                Some(var) => writeln!(f, "<mi{}>{}</mi>", var, letter),
                None => writeln!(f, "<mi>{}</mi>", letter),
            },
            Node::Operator(op) => writeln!(f, "<mo>{}</mo>", op.char()),
            Node::OperatorWithSpacing { op, left, right } => {
                match (left.is_finite(), right.is_finite()) {
                    (true, true) => writeln!(
                        f,
                        r#"<mo lspace="{}em" rspace="{}em">{}</mo>"#,
                        left,
                        right,
                        op.char()
                    ),
                    (true, false) => {
                        writeln!(f, r#"<mo lspace="{}em">{}</mo>"#, left, op.char())
                    }
                    (false, true) => {
                        writeln!(f, r#"<mo rspace="{}em">{}</mo>"#, right, op.char())
                    }
                    (false, false) => writeln!(f, "<mo>{}</mo>", op.char()),
                }
            }
            Node::MultiLetterIdent(letters, var) => match var {
                Some(var) => writeln!(f, "<mi{}>{}</mi>", var, letters,),
                None => writeln!(f, "<mi>{}</mi>", letters),
            },
            Node::Space(space) => writeln!(f, r#"<mspace width="{}em"/>"#, space,),
            Node::Subscript(base, sub) => {
                writeln!(f, "<msub>")?;
                base.render(f, child_indent)?;
                sub.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</msub>")
            }
            Node::Superscript(base, sup) => {
                writeln!(f, "<msup>")?;
                base.render(f, child_indent)?;
                sup.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</msup>")
            }
            Node::SubSup { target, sub, sup } => {
                writeln!(f, "<msubsup>")?;
                target.render(f, child_indent)?;
                sub.render(f, child_indent)?;
                sup.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</msubsup>")
            }
            Node::OverOp(op, acc, target) => {
                writeln!(f, "<mover>")?;
                target.render(f, child_indent)?;
                write_indent(f, child_indent)?;
                writeln!(f, r#"<mo accent="{}">{}</mo>"#, acc, op.char())?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mover>")
            }
            Node::UnderOp(op, acc, target) => {
                writeln!(f, "<munder>")?;
                target.render(f, child_indent)?;
                write_indent(f, child_indent)?;
                writeln!(f, r#"<mo accent="{}">{}</mo>"#, acc, op.char())?;
                write_indent(f, base_indent)?;
                writeln!(f, "</munder>")
            }
            Node::Overset { over, target } => {
                writeln!(f, "<mover>")?;
                target.render(f, child_indent)?;
                over.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mover>")
            }
            Node::Underset { under, target } => {
                writeln!(f, "<munder>")?;
                target.render(f, child_indent)?;
                under.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</munder>")
            }
            Node::UnderOver {
                target,
                under,
                over,
            } => {
                writeln!(f, "<munderover>")?;
                target.render(f, child_indent)?;
                under.render(f, child_indent)?;
                over.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</munderover>")
            }
            Node::Sqrt(content) => {
                writeln!(f, "<msqrt>")?;
                content.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</msqrt>")
            }
            Node::Root(degree, content) => {
                writeln!(f, "<mroot>")?;
                content.render(f, child_indent)?;
                degree.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mroot>")
            }
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    writeln!(f, "<mfrac{}{}>", lt, style)?;
                } else {
                    writeln!(f, "<mfrac{}>", lt)?;
                }
                num.render(f, child_indent)?;
                denom.render(f, child_indent)?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mfrac>")
            }
            Node::Row(vec) => {
                writeln!(f, "<mrow>")?;
                for node in vec.iter() {
                    node.render(f, child_indent)?;
                }
                write_indent(f, base_indent)?;
                writeln!(f, "</mrow>")
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
                writeln!(f, "<mrow>")?;
                write_indent(f, child_indent)?;
                writeln!(
                    f,
                    "<mo stretchy=\"true\" form=\"prefix\">{}</mo>",
                    open.char()
                )?;
                content.render(f, child_indent)?;
                write_indent(f, child_indent)?;
                writeln!(
                    f,
                    "<mo stretchy=\"true\" form=\"postfix\">{}</mo>",
                    close.char(),
                )?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mrow>")
            }
            Node::StretchedOp(stretchy, op) => {
                writeln!(f, r#"<mo stretchy="{}">{}</mo>"#, stretchy, op.char())
            }
            Node::Paren(op) => writeln!(f, r#"<mo stretchy="false">{}</mo>"#, op.char()),
            Node::SizedParen { size, paren } => writeln!(
                f,
                r#"<mrow><mo maxsize="{0}" minsize="{0}">{1}</mro></mrow>"#,
                size,
                paren.char()
            ),
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => writeln!(f, "<mi{}>{}&#x0338;</mi>", var, x),
                    None => writeln!(f, "<mi>{}&#x0338;</mi>", x),
                },
                Node::Operator(x) => writeln!(f, "<mo>{}&#x0338;</mo>", x.char()),
                n => n.render(f, base_indent),
            },
            Node::Table(content, align) => {
                let i1 = child_indent;
                let i2 = child_indent.saturating_add(1);
                let i3 = i2.saturating_add(1);
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
                writeln!(f, "<mtable>")?;
                write_indent(f, i1)?;
                writeln!(f, "<mtr>")?;
                write_indent(f, i2)?;
                writeln!(f, "{}", odd_col)?;
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            write_indent(f, i2)?;
                            writeln!(f, "</mtd>")?;
                            col += 1;
                            if j < total_len {
                                write_indent(f, i2)?;
                                writeln!(f, "{}", if col % 2 == 0 { even_col } else { odd_col },)?;
                            }
                        }
                        Node::RowSeparator => {
                            write_indent(f, i2)?;
                            writeln!(f, "</mtd>")?;
                            write_indent(f, i1)?;
                            writeln!(f, "</mtr>")?;
                            if j < total_len {
                                write_indent(f, i1)?;
                                writeln!(f, "<mtr>")?;
                                write_indent(f, i2)?;
                                writeln!(f, "{}", odd_col)?;
                            }
                            col = 1;
                        }
                        node => {
                            node.render(f, i3)?;
                        }
                    }
                }
                write_indent(f, i2)?;
                writeln!(f, "</mtd>")?;
                write_indent(f, i1)?;
                writeln!(f, "</mtr>")?;
                write_indent(f, base_indent)?;
                writeln!(f, "</mtable>")
            }
            Node::Text(text) => writeln!(f, "<mtext>{}</mtext>", text),
            Node::ColumnSeparator | Node::RowSeparator => Ok(()),
        }
    }
}

fn write_indent(f: &mut fmt::Formatter<'_>, indent_num: usize) -> fmt::Result {
    for _ in 0..indent_num {
        write!(f, "{}", INDENT)?;
    }
    Ok(())
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
            (Node::Number("3.14".to_owned()), "<mn>3.14</mn>\n"),
            (Node::SingleLetterIdent('x', None), "<mi>x</mi>\n"),
            (Node::SingleLetterIdent('α', None), "<mi>α</mi>\n"),
            (
                Node::SingleLetterIdent('あ', Some(MathVariant::Normal)),
                "<mi mathvariant=\"normal\">あ</mi>\n",
            ),
        ];
        for (problem, answer) in problems.iter() {
            assert_eq!(&format!("{}", problem), answer);
        }
    }
}
