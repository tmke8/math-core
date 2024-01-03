extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
use bumpalo::Bump;
use latex2mmlc::{latex_to_mathml, Display};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert(content: &str, block: bool, pretty: bool) -> Result<JsValue, JsValue> {
    let alloc = &Bump::new();
    let result = latex_to_mathml(
        alloc,
        content,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        pretty,
    );
    match result {
        Ok(result) => Ok(JsValue::from_str(&result)),
        Err(e) => Err(JsValue::from_str(&e.string(alloc))),
    }
}
