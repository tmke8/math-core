use crate::{arena::{Arena, NodeList}, ast::Node, attribute::Align, error::{LatexError, LatexErrKind}, ops};


#[inline]
pub fn parse_env<'source>(
    arena: &mut Arena<'source>,
    name: &'source str,
    content: NodeList,
    loc: usize,
) -> Result<Node<'source>, LatexError<'source>> {
    let node = match name {
        "align" | "align*" | "aligned" => Node::Table(content, Align::Alternating),
        "cases" => {
            let content = arena.push(Node::Table(content, Align::Left));
            Node::Fenced {
                open: ops::LEFT_CURLY_BRACKET,
                close: ops::NULL,
                content,
                style: None,
            }
        }
        "matrix" => Node::Table(content, Align::Center),
        matrix_variant @ ("pmatrix" | "bmatrix" | "vmatrix") => {
            let content = arena.push(Node::Table(content, Align::Center));
            let (open, close) = match matrix_variant {
                "pmatrix" => (ops::LEFT_PARENTHESIS, ops::RIGHT_PARENTHESIS),
                "bmatrix" => (ops::LEFT_SQUARE_BRACKET, ops::RIGHT_SQUARE_BRACKET),
                "vmatrix" => (ops::VERTICAL_LINE, ops::VERTICAL_LINE),
                // SAFETY: `matrix_variant` is one of the three strings above.
                _ => unsafe { std::hint::unreachable_unchecked() },
            };
            Node::Fenced {
                open,
                close,
                content,
                style: None,
            }
        }
        _ => {
            return Err(LatexError(loc, LatexErrKind::UnknownEnvironment(name)));
        }
    };
    Ok(node)
}
