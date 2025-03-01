use latex2mmlc::token::{TokLoc, Token};
use mathml_renderer::ast::Node;

const WORD: usize = std::mem::size_of::<usize>();

#[test]
fn test_struct_sizes() {
    assert!(std::mem::size_of::<Token>() <= 3 * WORD, "size of Token");
    assert!(std::mem::size_of::<TokLoc>() <= 4 * WORD, "size of TokLoc");
    assert!(std::mem::size_of::<Node>() <= 4 * WORD, "size of Node");
}
