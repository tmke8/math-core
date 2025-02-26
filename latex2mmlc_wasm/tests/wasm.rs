use wasm_bindgen_test::*;

use mathml_renderer::arena::NodeList;
use mathml_renderer::ast::Node;
use latex2mmlc::token::{TokLoc, Token};

#[wasm_bindgen_test]
fn check_sizes() {
    assert_eq!(std::mem::size_of::<Token>(), 12, "size of Token");
    assert_eq!(std::mem::size_of::<TokLoc>(), 16, "size of TokLoc");
    assert_eq!(std::mem::size_of::<Node>(), 16, "size of Node");
    assert_eq!(std::mem::size_of::<NodeList>(), 4, "size of Node");
}
