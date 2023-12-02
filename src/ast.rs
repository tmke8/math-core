use crate::ops::Op;

use super::attribute::{Accent, Align, DisplayStyle, LineThickness, MathVariant};
use std::fmt::{self, Alignment};

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
    Undefined(String),
}

const INDENT: usize = 4;

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compute the base indent for the node.
        // We abuse the width field of the formatter to store the base indent.
        let b = match (f.width(), f.align()) {
            (Some(width), Some(Alignment::Right)) => width,
            _ => 0,
        };
        // Compute the indent for the children of the node.
        let i = b.saturating_add(INDENT);

        match self {
            Node::Number(number) => writeln!(f, "{:b$}<mn>{}</mn>", "", number),
            Node::SingleLetterIdent(letter, var) => match var {
                Some(var) => writeln!(f, "{:b$}<mi{}>{}</mi>", "", var, letter),
                None => writeln!(f, "{:b$}<mi>{}</mi>", "", letter),
            },
            Node::Operator(op) => writeln!(f, "{:b$}<mo>{}</mo>", "", op.char()),
            Node::OperatorWithSpacing { op, left, right } => {
                match (left.is_finite(), right.is_finite()) {
                    (true, true) => writeln!(
                        f,
                        r#"{:b$}<mo lspace="{}em" rspace="{}em">{}</mo>"#,
                        "",
                        left,
                        right,
                        op.char()
                    ),
                    (true, false) => {
                        writeln!(f, r#"{:b$}<mo lspace="{}em">{}</mo>"#, "", left, op.char())
                    }
                    (false, true) => {
                        writeln!(f, r#"{:b$}<mo rspace="{}em">{}</mo>"#, "", right, op.char())
                    }
                    (false, false) => writeln!(f, "{:b$}<mo>{}</mo>", "", op.char()),
                }
            }
            Node::MultiLetterIdent(letters, var) => match var {
                Some(var) => writeln!(f, "{:b$}<mi{}>{}</mi>", "", var, letters,),
                None => writeln!(f, "{:b$}<mi>{}</mi>", "", letters),
            },
            Node::Space(space) => writeln!(f, r#"{:b$}<mspace width="{}em"/>"#, "", space,),
            Node::Subscript(base, sub) => writeln!(
                f,
                "{:b$}<msub>\n{:>i$}{:>i$}{:b$}</msub>",
                "", base, sub, "",
            ),
            Node::Superscript(base, sup) => writeln!(
                f,
                "{:b$}<msup>\n{:>i$}{:>i$}{:b$}</msup>",
                "", base, sup, "",
            ),
            Node::SubSup { target, sub, sup } => {
                writeln!(
                    f,
                    "{:b$}<msubsup>\n{:>i$}{:>i$}{:>i$}{:b$}</msubsup>",
                    "", target, sub, sup, "",
                )
            }
            Node::OverOp(op, acc, target) => writeln!(
                f,
                r#"{:b$}<mover>
{:>i$}{:i$}<mo accent="{}">{}</mo>
{:b$}</mover>"#,
                "",
                target,
                "",
                acc,
                op.char(),
                "",
            ),
            Node::UnderOp(op, acc, target) => writeln!(
                f,
                r#"{:b$}<munder>
{:>i$}{:i$}<mo accent="{}">{}</mo>
{:b$}</munder>"#,
                "",
                target,
                "",
                acc,
                op.char(),
                "",
            ),
            Node::Overset { over, target } => writeln!(
                f,
                "{:b$}<mover>\n{:>i$}{:>i$}{:b$}</mover>",
                "", target, over, ""
            ),
            Node::Underset { under, target } => writeln!(
                f,
                "{:b$}<munder>\n{:>i$}{:>i$}{:b$}</munder>",
                "", target, under, ""
            ),
            Node::UnderOver {
                target,
                under,
                over,
            } => writeln!(
                f,
                "{:b$}<munderover>\n{:>i$}{:>i$}{:>i$}{:b$}</munderover>",
                "", target, under, over, ""
            ),
            Node::Sqrt(content) => {
                writeln!(f, "{:b$}<msqrt>\n{:>i$}{:b$}</msqrt>", "", content, "",)
            }
            Node::Root(degree, content) => writeln!(
                f,
                "{:b$}<mroot>\n{:>i$}{:>i$}{:b$}</mroot>",
                "", content, degree, "",
            ),
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    writeln!(
                        f,
                        "{:b$}<mfrac{}{}>\n{:>i$}{:>i$}{:b$}</mfrac>",
                        "", lt, style, num, denom, ""
                    )
                } else {
                    writeln!(
                        f,
                        "{:b$}<mfrac{}>\n{:>i$}{:>i$}{:b$}</mfrac>",
                        "", lt, num, denom, ""
                    )
                }
            }
            Node::Row(vec) => {
                writeln!(f, "{:b$}<mrow>", "",)?;
                for node in vec.iter() {
                    write!(f, "{:>i$}", node)?;
                }
                writeln!(f, "{:b$}</mrow>", "")
            }
            Node::PseudoRow(vec) => {
                for node in vec.iter() {
                    write!(f, "{:>b$}", node)?;
                }
                Ok(())
            }
            Node::Fenced {
                open,
                close,
                content,
            } => {
                writeln!(
                    f,
                    r#"{:b$}<mrow>
{:i$}<mo stretchy="true" form="prefix">{}</mo>
{:>i$}{:i$}<mo stretchy="true" form="postfix">{}</mo>
{:b$}</mrow>"#,
                    "", "", open.0, content, "", close.0, ""
                )
            }
            Node::StretchedOp(stretchy, op) => {
                writeln!(
                    f,
                    r#"{:b$}<mo stretchy="{}">{}</mo>"#,
                    "",
                    stretchy,
                    op.char()
                )
            }
            Node::Paren(op) => writeln!(f, r#"{:b$}<mo stretchy="false">{}</mo>"#, "", op.char()),
            Node::SizedParen { size, paren } => writeln!(
                f,
                r#"{0:b$}<mrow><mo maxsize="{1}" minsize="{1}">{2}</mro></mrow>"#,
                "", size, paren.0
            ),
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => writeln!(f, "<mi{}>{}&#x0338;</mi>", var, x),
                    None => writeln!(f, "<mi>{}&#x0338;</mi>", x),
                },
                Node::Operator(x) => writeln!(f, "<mo>{}&#x0338;</mo>", x.char()),
                n => write!(f, "{}", n),
            },
            Node::Table(content, align) => {
                let i2 = i.saturating_add(INDENT);
                let i3 = i2.saturating_add(INDENT);
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
                writeln!(
                    f,
                    "{:b$}<mtable>\n{:i$}<mtr>\n{:i2$}{}",
                    "", "", "", odd_col,
                )?;
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            writeln!(f, "{:i2$}</mtd>", "")?;
                            col += 1;
                            if j < total_len {
                                writeln!(
                                    f,
                                    "{:i2$}{}",
                                    "",
                                    if col % 2 == 0 { even_col } else { odd_col },
                                )?;
                            }
                        }
                        Node::RowSeparator => {
                            writeln!(f, "{:i2$}</mtd>\n{:i$}</mtr>", "", "")?;
                            if j < total_len {
                                writeln!(f, "{:i$}<mtr>\n{:i2$}{}", "", "", odd_col,)?;
                            }
                            col = 1;
                        }
                        node => {
                            write!(f, "{:>i3$}", node)?;
                        }
                    }
                }
                writeln!(f, "{:i2$}</mtd>\n{:i$}</mtr>\n{:b$}</mtable>", "", "", "")
            }
            Node::Text(text) => writeln!(f, "{:b$}<mtext>{}</mtext>", "", text),
            node => writeln!(
                f,
                "{:b$}<merror><mtext>Parse error: {:?}</mtext></merror>",
                "", node
            ),
        }
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
