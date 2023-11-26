use super::attribute::{Accent, LineThickness, MathVariant, DisplayStyle};
use std::fmt;

/// AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Number(String),
    Letter(char, MathVariant),
    Operator(char),
    Function(String, Option<Box<Node>>),
    Space(f32),
    Subscript(Box<Node>, Box<Node>),
    Superscript(Box<Node>, Box<Node>),
    SubSup {
        target: Box<Node>,
        sub: Box<Node>,
        sup: Box<Node>,
    },
    OverOp(char, Accent, Box<Node>),
    UnderOp(char, Accent, Box<Node>),
    Overset {
        over: Box<Node>,
        target: Box<Node>,
    },
    Underset {
        under: Box<Node>,
        target: Box<Node>,
    },
    Under(Box<Node>, Box<Node>),
    UnderOver {
        target: Box<Node>,
        under: Box<Node>,
        over: Box<Node>,
    },
    Sqrt(Option<Box<Node>>, Box<Node>),
    Frac(Box<Node>, Box<Node>, LineThickness, Option<DisplayStyle>),
    Row(Vec<Node>),
    Fenced {
        open: &'static str,
        close: &'static str,
        content: Box<Node>,
    },
    StretchedOp(bool, String),
    OtherOperator(&'static str),
    Paren(&'static str),
    SizedParen {
        size: &'static str,
        paren: &'static str,
    },
    Text(String),
    Matrix(Vec<Node>),
    Aligned(Vec<Node>),
    NewColumn,
    NewRow,
    Slashed(Box<Node>),
    Undefined(String),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Number(number) => write!(f, "<mn>{}</mn>", number),
            Node::Letter(letter, var) => write!(f, "<mi{}>{}</mi>", var, letter),
            Node::Operator(op) => {
                if op == &'∂' {
                    write!(f, r#"<mo mathvariant="italic">∂</mo>"#)
                } else {
                    write!(f, r#"<mo>{}</mo>"#, op)
                }
            }
            Node::Function(fun, arg) => match arg {
                Some(arg) => write!(f, "<mi>{}</mi><mo>&#x2061;</mo>{}", fun, arg),
                None => write!(f, "<mi>{}</mi>", fun),
            },
            Node::Space(space) => write!(f, r#"<mspace width="{}em"/>"#, space),
            Node::Subscript(a, b) => write!(f, "<msub>{}{}</msub>", a, b),
            Node::Superscript(a, b) => write!(f, "<msup>{}{}</msup>", a, b),
            Node::SubSup { target, sub, sup } => {
                write!(f, "<msubsup>{}{}{}</msubsup>", target, sub, sup)
            }
            Node::OverOp(op, acc, target) => write!(
                f,
                r#"<mover>{}<mo accent="{}">{}</mo></mover>"#,
                target, acc, op
            ),
            Node::UnderOp(op, acc, target) => write!(
                f,
                r#"<munder>{}<mo accent="{}">{}</mo></munder>"#,
                target, acc, op
            ),
            Node::Overset { over, target } => write!(f, r#"<mover>{}{}</mover>"#, target, over),
            Node::Underset { under, target } => {
                write!(f, r#"<munder>{}{}</munder>"#, target, under)
            }
            Node::Under(target, under) => write!(f, r#"<munder>{}{}</munder>"#, target, under),
            Node::UnderOver {
                target,
                under,
                over,
            } => write!(f, r#"<munderover>{}{}{}</munderover>"#, target, under, over),
            Node::Sqrt(degree, content) => match degree {
                Some(deg) => write!(f, "<mroot>{}{}</mroot>", content, deg),
                None => write!(f, "<msqrt>{}</msqrt>", content),
            },
            Node::Frac(num, denom, lt, style) => {
                if let Some(style) = style {
                    write!(f, "<mfrac{}{}>{}{}</mfrac>", lt, style, num, denom)
                } else {
                    write!(f, "<mfrac{}>{}{}</mfrac>", lt, num, denom)
                }
            }
            Node::Row(vec) => write!(
                f,
                "<mrow>{}</mrow>",
                vec.iter()
                    .map(|node| format!("{}", node))
                    .collect::<String>()
            ),
            Node::Fenced {
                open,
                close,
                content,
            } => {
                write!(
                    f,
                    r#"<mrow><mo stretchy="true" form="prefix">{}</mo>{}<mo stretchy="true" form="postfix">{}</mo></mrow>"#,
                    open, content, close
                )
            }
            Node::StretchedOp(stretchy, op) => {
                write!(f, r#"<mo stretchy="{}">{}</mo>"#, stretchy, op)
            }
            Node::Paren(paren) => write!(f, r#"<mo stretchy="false">{}</mo>"#, paren),
            Node::OtherOperator(op) => write!(f, "<mo>{}</mo>", op),
            Node::SizedParen { size, paren } => write!(
                f,
                r#"<mrow><mo maxsize="{0}" minsize="{0}">{1}</mro></mrow>"#,
                size, paren
            ),
            Node::Slashed(node) => match &**node {
                Node::Letter(x, var) => write!(f, "<mi mathvariant=\"{}\">{}&#x0338;</mi>", var, x),
                Node::Operator(x) => write!(f, "<mo>{}&#x0338;</mo>", x),
                n => write!(f, "{}", n),
            },
            Node::Matrix(content) => {
                let mut mathml = "<mtable><mtr><mtd>".to_string();
                for (i, node) in content.iter().enumerate() {
                    match node {
                        Node::NewColumn => {
                            mathml.push_str("</mtd>");
                            if i < content.len() {
                                mathml.push_str("<mtd>")
                            }
                        }
                        Node::NewRow => {
                            mathml.push_str("</mtd></mtr>");
                            if i < content.len() {
                                mathml.push_str("<mtr><mtd>")
                            }
                        }
                        node => {
                            mathml = format!("{}{}", mathml, node);
                        }
                    }
                }
                mathml.push_str("</mtd></mtr></mtable>");

                write!(f, "{}", mathml)
            }
            Node::Aligned(content) => {
                let mut mathml =
                    "<mtable>\n<mtr><mtd style=\"text-align: right; padding-right: 0\">"
                        .to_string();
                let mut col = 0;
                let total_len = content.len();
                for (i, node) in content.iter().enumerate() {
                    match node {
                        Node::NewColumn => {
                            mathml.push_str("</mtd>");
                            col += 1;
                            if i < total_len {
                                mathml.push_str(if col % 2 == 0 {
                                    "<mtd style=\"text-align: right; padding-right: 0\">"
                                } else {
                                    "<mtd style=\"text-align: left; padding-left: 0\">"
                                })
                            }
                        }
                        Node::NewRow => {
                            mathml.push_str("</mtd></mtr>");
                            if i < total_len {
                                mathml.push_str(
                                    "\n<mtr><mtd style=\"text-align: right; padding-right: 0\">",
                                )
                            }
                            col = 0;
                        }
                        node => {
                            mathml = format!("{}{}", mathml, node);
                        }
                    }
                }
                mathml.push_str("</mtd></mtr>\n</mtable>");

                write!(f, "{}", mathml)
            }
            Node::Text(text) => write!(f, "<mtext>{}</mtext>", text),
            node => write!(f, "<merror><mtext>[Parse error: {:?}]</mtext></merror>", node),
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
            (Node::Letter('x', MathVariant::Default), "<mi>x</mi>"),
            (Node::Letter('α', MathVariant::Default), "<mi>α</mi>"),
            (
                Node::Letter('あ', MathVariant::Normal),
                r#"<mi mathvariant="normal">あ</mi>"#,
            ),
            (
                Node::Row(vec![Node::Operator('+'), Node::Operator('-')]),
                r"<mrow><mo>+</mo><mo>-</mo></mrow>",
            ),
        ];
        for (problem, answer) in problems.iter() {
            assert_eq!(&format!("{}", problem), answer);
        }
    }
}
