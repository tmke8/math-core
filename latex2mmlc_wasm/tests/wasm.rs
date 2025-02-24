use wasm_bindgen_test::*;

use latex2mmlc::parse::Bounds;
use latex2mmlc::token::{TokLoc, Token};
use mathml_renderer::ast::Node;

#[wasm_bindgen_test]
fn test_sizes() {
    assert_eq!(std::mem::size_of::<Token>(), 12, "size of Token");
    assert_eq!(std::mem::size_of::<TokLoc>(), 16, "size of TokLoc");
    assert_eq!(std::mem::size_of::<Node>(), 16, "size of Node");
    assert_eq!(std::mem::size_of::<Option<Bounds>>(), 12, "size of Bounds");
}
