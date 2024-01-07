extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use latex2mmlc::{latex_to_mathml, Display};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn convert(content: &str, block: bool, pretty: bool) -> Result<JsValue, JsValue> {
    match latex_to_mathml(
        content,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        pretty,
    ) {
        Ok(result) => Ok(JsValue::from_str(&result)),
        Err(e) => Err(JsValue::from_str(&e.string())),
    }
}
