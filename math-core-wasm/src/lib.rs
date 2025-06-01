extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use math_core::{Config, Converter, Display};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    pub error_message: JsValue,
    pub location: u32,
}

#[wasm_bindgen]
extern "C" {
    pub type JsConfig;

    #[wasm_bindgen(method, getter)]
    fn pretty(this: &JsConfig) -> bool;
}

#[wasm_bindgen]
pub fn convert(content: &str, block: bool, js_config: JsConfig) -> Result<JsValue, LatexError> {
    // This is the poor man's `serde_wasm_bindgen::from_value`.
    let config = Config {
        pretty: js_config.pretty(),
        ..Default::default()
    };
    let mut converter = Converter::new(config);

    match converter.latex_to_mathml(
        content,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
    ) {
        Ok(result) => Ok(JsValue::from_str(&result)),
        Err(e) => Err(LatexError {
            error_message: JsValue::from_str(&e.1.string()),
            location: e.0 as u32,
        }),
    }
}
