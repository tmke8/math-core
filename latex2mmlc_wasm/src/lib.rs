extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
use latex2mmlc::{latex_to_mathml, Display};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert(content: &str, block: bool) -> Result<String, JsValue> {
    match latex_to_mathml(
        content,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
    ) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Conversion failed: {:?}", e))),
    }
}
