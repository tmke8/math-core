use crate::token::Op;

use super::attribute::{Accent, DisplayStyle, LineThickness, MathVariant};
use std::fmt::{self, Alignment};

/// AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Number(String),
    SingleLetterIdent(char, Option<MathVariant>),
    Operator(Op),
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
    Table(Vec<Node>),
    AlignedTable(Vec<Node>),
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
            Node::Number(number) => write!(f, "{:b$}<mn>{}</mn>", "", number),
            Node::SingleLetterIdent(letter, var) => match var {
                Some(var) => write!(f, "{:b$}<mi{}>{}</mi>", "", var, letter),
                None => write!(f, "{:b$}<mi>{}</mi>", "", letter),
            },
            Node::Operator(Op(op)) => write!(f, "{:b$}<mo>{}</mo>", "", op),
            Node::MultiLetterIdent(letters, var) => match var {
                Some(var) => write!(f, "{:b$}<mi{}>{}</mi>", "", var, letters,),
                None => write!(f, "{:b$}<mi>{}</mi>", "", letters),
            },
            Node::Space(space) => write!(f, r#"{:b$}<mspace width="{}em"/>"#, "", space,),
            Node::Subscript(base, sub) => write!(
                f,
                "{:b$}<msub>\n{:>i$}\n{:>i$}\n{:b$}</msub>",
                "", base, sub, "",
            ),
            Node::Superscript(base, sup) => write!(
                f,
                "{:b$}<msup>\n{:>i$}\n{:>i$}\n{:b$}</msup>",
                "", base, sup, "",
            ),
            Node::SubSup { target, sub, sup } => {
                write!(
                    f,
                    "{:b$}<msubsup>\n{:>i$}\n{:>i$}\n{:>i$}\n{:b$}</msubsup>",
                    "", target, sub, sup, "",
                )
            }
            Node::OverOp(Op(c), acc, target) => write!(
                f,
                "{:b$}<mover>\n{:>i$}\n{:i$}<mo accent=\"{}\">{}</mo>\n{:b$}</mover>",
                "", target, "", acc, c, "",
            ),
            Node::UnderOp(Op(c), acc, target) => write!(
                f,
                "{:b$}<munder>\n{:>i$}\n{:i$}<mo accent=\"{}\">{}</mo>\n{:b$}</munder>",
                "", target, "", acc, c, "",
            ),
            Node::Overset { over, target } => write!(
                f,
                "{:b$}<mover>\n{:>i$}\n{:>i$}\n{:b$}</mover>",
                "", target, over, ""
            ),
            Node::Underset { under, target } => write!(
                f,
                "{:b$}<munder>\n{:>i$}\n{:>i$}\n{:b$}</munder>",
                "", target, under, ""
            ),
            Node::UnderOver {
                target,
                under,
                over,
            } => write!(
                f,
                "{:b$}<munderover>\n{:>i$}\n{:>i$}\n{:>i$}\n{:b$}</munderover>",
                "", target, under, over, ""
            ),
            Node::Sqrt(content) => {
                write!(f, "{:b$}<msqrt>\n{:>i$}\n{:b$}</msqrt>", "", content, "",)
            }
            Node::Root(degree, content) => write!(
                f,
                "{:b$}<mroot>\n{:>i$}\n{:>i$}\n{:b$}</mroot>",
                "", content, degree, "",
            ),
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    write!(
                        f,
                        "{:b$}<mfrac{}{}>\n{:>i$}\n{:>i$}\n{:b$}</mfrac>",
                        "", lt, style, num, denom, ""
                    )
                } else {
                    write!(
                        f,
                        "{:b$}<mfrac{}>\n{:>i$}\n{:>i$}\n{:b$}</mfrac>",
                        "", lt, num, denom, ""
                    )
                }
            }
            Node::Row(vec) => {
                write!(f, "{:b$}<mrow>\n", "",)?;
                for node in vec.iter() {
                    write!(f, "{:>i$}\n", node)?;
                }
                write!(f, "{:b$}</mrow>", "")
            }
            Node::Fenced {
                open,
                close,
                content,
            } => {
                write!(
                    f,
                    r#"{:b$}<mrow>
{:i$}<mo stretchy="true" form="prefix">{}</mo>
{:>i$}
{:i$}<mo stretchy="true" form="postfix">{}</mo>
{:b$}</mrow>"#,
                    "", "", open.0, content, "", close.0, ""
                )
            }
            Node::StretchedOp(stretchy, Op(c)) => {
                write!(f, "{:b$}<mo stretchy=\"{}\">{}</mo>", "", stretchy, c)
            }
            Node::Paren(Op(c)) => write!(f, "{:b$}<mo stretchy=\"false\">{}</mo>", "", c),
            Node::SizedParen { size, paren } => write!(
                f,
                r#"{0:b$}<mrow><mo maxsize="{1}" minsize="{1}">{2}</mro></mrow>"#,
                "", size, paren.0
            ),
            Node::Slashed(node) => match &**node {
                Node::SingleLetterIdent(x, var) => match var {
                    Some(var) => write!(f, "<mi{}>{}&#x0338;</mi>", var, x),
                    None => write!(f, "<mi>{}&#x0338;</mi>", x),
                },
                Node::Operator(Op(x)) => write!(f, "<mo>{}&#x0338;</mo>", x),
                n => write!(f, "{}", n),
            },
            Node::Table(content) => {
                let i2 = i.saturating_add(INDENT);
                let i3 = i2.saturating_add(INDENT);
                write!(f, "{:b$}<mtable>\n{:i$}<mtr>\n{:i2$}<mtd>\n", "", "", "")?;
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            write!(f, "{:i2$}</mtd>\n", "")?;
                            if j < total_len {
                                write!(f, "{:i2$}<mtd>\n", "")?;
                            }
                        }
                        Node::RowSeparator => {
                            write!(f, "{:i2$}</mtd>\n{:i$}</mtr>\n", "", "")?;
                            if j < total_len {
                                write!(f, "{:i$}<mtr>\n{:i2$}<mtd>\n", "", "",)?;
                            }
                        }
                        node => {
                            write!(f, "{:>i3$}\n", node)?;
                        }
                    }
                }
                write!(f, "{:i2$}</mtd>\n{:i$}</mtr>\n{:b$}</mtable>", "", "", "")
            }
            Node::AlignedTable(content) => {
                let i2 = i.saturating_add(INDENT);
                let i3 = i2.saturating_add(INDENT);
                write!(f, "{:b$}<mtable>\n{:i$}<mtr>\n{:i2$}<mtd style=\"text-align: right; padding-right: 0\">\n", "", "", "")?;
                let mut col = 0;
                let total_len = content.len();
                for (j, node) in content.iter().enumerate() {
                    match node {
                        Node::ColumnSeparator => {
                            write!(f, "{:i2$}</mtd>\n", "")?;
                            col += 1;
                            if j < total_len {
                                write!(
                                    f,
                                    "{:i2$}{}",
                                    "",
                                    if col % 2 == 0 {
                                        "<mtd style=\"text-align: right; padding-right: 0\">\n"
                                    } else {
                                        "<mtd style=\"text-align: left; padding-left: 0\">\n"
                                    },
                                )?;
                            }
                        }
                        Node::RowSeparator => {
                            write!(f, "{:i2$}</mtd>\n{:i$}</mtr>\n", "", "")?;
                            if j < total_len {
                                write!(
                                    f,
                                    "{:i$}<mtr>\n{:i2$}<mtd style=\"text-align: right; padding-right: 0\">\n",
                                    "", "",
                                )?;
                            }
                            col = 0;
                        }
                        node => {
                            write!(f, "{:>i3$}\n", node)?;
                        }
                    }
                }
                write!(f, "{:i2$}</mtd>\n{:i$}</mtr>\n{:b$}</mtable>", "", "", "")
            }
            Node::Text(text) => write!(f, "{:b$}<mtext>{}</mtext>", "", text),
            node => write!(
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
            (Node::Number("3.14".to_owned()), "<mn>3.14</mn>"),
            (Node::SingleLetterIdent('x', None), "<mi>x</mi>"),
            (Node::SingleLetterIdent('α', None), "<mi>α</mi>"),
            (
                Node::SingleLetterIdent('あ', Some(MathVariant::Normal)),
                r#"<mi mathvariant="normal">あ</mi>"#,
            ),
        ];
        for (problem, answer) in problems.iter() {
            assert_eq!(&format!("{}", problem), answer);
        }
    }
}
