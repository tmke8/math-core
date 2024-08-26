extern crate alloc;

/*
#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };
*/

#[cfg(target_arch = "wasm32")]
use lol_alloc::FailAllocator;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: FailAllocator = FailAllocator;

use latex2mmlc::{latex_to_mathml, Arena, Display};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    pub error_message: JsValue,
    pub location: u32,
}

#[wasm_bindgen]
pub fn convert(content: &str, block: bool, pretty: bool) -> Result<JsValue, LatexError> {
    let arena = Arena::new();
    match latex_to_mathml(
        content,
        &arena,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        pretty,
    ) {
        Ok(result) => Ok(JsValue::from_str(result)),
        Err(e) => Err(LatexError {
            error_message: JsValue::from_str(e.1.string(&arena)),
            location: e.0 as u32,
        }),
    }
}
