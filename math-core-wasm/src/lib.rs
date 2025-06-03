extern crate alloc;

use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use js_sys::{Array, Map};
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

    #[wasm_bindgen(method, getter)]
    fn macros(this: &JsConfig) -> Map;
}

#[wasm_bindgen]
pub fn convert(content: &str, block: bool, js_config: &JsConfig) -> Result<JsValue, LatexError> {
    // This is the poor man's `serde_wasm_bindgen::from_value`.
    let macro_map = js_config.macros();
    let mut macros = HashMap::with_capacity(macro_map.size() as usize);
    let macro_iter = macro_map.entries();
    loop {
        let Ok(entry) = macro_iter.next() else {
            break; // Exit the loop on error.
        };
        if entry.done() {
            break; // Exit the loop when there are no more entries.
        }
        let Ok(value) = entry.value().dyn_into::<Array>() else {
            continue; // Skip if the value is not an Array.
        };
        let Some(key) = value.get(0).as_string() else {
            continue; // Skip if the first element is not a string.
        };
        let Some(value) = value.get(1).as_string() else {
            continue; // Skip if the second element is not a string.
        };
        macros.insert(key, value);
    }
    let config = Config {
        pretty: js_config.pretty(),
        macros,
        ..Default::default()
    };
    let mut converter = Converter::new(&config).map_err(|e| {
        let error = e.1.string() + "\n(This is an error from a custom macro.)";
        LatexError {
            error_message: JsValue::from_str(&error),
            location: e.0 as u32,
        }
    })?;

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
