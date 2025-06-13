extern crate alloc;

use std::sync::RwLock;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use js_sys::{Array, Map};
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
use rustc_hash::FxHashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    pub message: JsValue,
    pub location: u32,
}

#[wasm_bindgen]
extern "C" {
    pub type JsConfig;

    #[wasm_bindgen(method, getter)]
    fn prettyPrint(this: &JsConfig) -> bool;

    #[wasm_bindgen(method, getter)]
    fn macros(this: &JsConfig) -> Map;
}

thread_local! {
    static LOCK_ERR_MGS: JsValue = JsValue::from_str("Couldn't get lock");
}

static LATEX_TO_MATHML: RwLock<LatexToMathML> = RwLock::new(LatexToMathML::const_default());

#[wasm_bindgen]
pub fn set_config(js_config: &JsConfig) -> Result<(), LatexError> {
    // This is the poor man's `serde_wasm_bindgen::from_value`.
    let macro_map = js_config.macros();
    let mut macros =
        FxHashMap::with_capacity_and_hasher(macro_map.size() as usize, Default::default());
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
    let config = MathCoreConfig {
        pretty_print: js_config.prettyPrint(),
        macros,
        ..Default::default()
    };
    let converter = LatexToMathML::new(&config).map_err(|e| LatexError {
        message: JsValue::from_str(&e.1.string()),
        location: e.0 as u32,
    })?;
    let mut global_converter = LATEX_TO_MATHML.write().map_err(|_| LatexError {
        message: LOCK_ERR_MGS.with(Clone::clone),
        location: 0,
    })?;
    *global_converter = converter;
    Ok(())
}

#[wasm_bindgen]
pub fn convert(content: &str, block: bool) -> Result<JsValue, LatexError> {
    match LATEX_TO_MATHML
        .read()
        .map_err(|_| LatexError {
            message: LOCK_ERR_MGS.with(Clone::clone),
            location: 0,
        })?
        .convert_with_local_counter(
            content,
            if block {
                MathDisplay::Block
            } else {
                MathDisplay::Inline
            },
        ) {
        Ok(result) => Ok(JsValue::from_str(&result)),
        Err(e) => Err(LatexError {
            message: JsValue::from_str(&e.1.string()),
            location: e.0 as u32,
        }),
    }
}
