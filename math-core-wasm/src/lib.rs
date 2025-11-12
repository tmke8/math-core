extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use js_sys::{Array, Map};
use math_core::{MathDisplay, PrettyPrint};
use rustc_hash::FxHashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    pub message: JsValue,
    pub location: u32,
}

#[wasm_bindgen]
pub struct LatexToMathML {
    inner: math_core::LatexToMathML,
}

#[wasm_bindgen(typescript_custom_section)]
const ITEXT_STYLE: &'static str = r#"
interface MathCoreOptions {
    prettyPrint: "never" | "always" | "auto";
    macros: Map<string, string>;
    xmlNamespace: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "MathCoreOptions")]
    pub type MathCoreOptions;

    #[wasm_bindgen(method, getter)]
    fn prettyPrint(this: &MathCoreOptions) -> String;

    #[wasm_bindgen(method, getter)]
    fn macros(this: &MathCoreOptions) -> Map;

    #[wasm_bindgen(method, getter)]
    fn xmlNamespace(this: &MathCoreOptions) -> bool;
}

#[wasm_bindgen]
impl LatexToMathML {
    #[wasm_bindgen(constructor)]
    pub fn new(js_config: &MathCoreOptions) -> Result<LatexToMathML, LatexError> {
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
        let pretty_print = match js_config.prettyPrint().as_str() {
            "always" => PrettyPrint::Always,
            "never" => PrettyPrint::Never,
            "auto" => PrettyPrint::Auto,
            _ => {
                return Err(LatexError {
                    message: JsValue::from_str("Invalid value for prettyPrint"),
                    location: 0,
                });
            }
        };
        let xml_namespace = js_config.xmlNamespace();
        let config = math_core::MathCoreConfig {
            pretty_print,
            macros,
            xml_namespace,
            ..Default::default()
        };
        let convert = math_core::LatexToMathML::new(&config).map_err(|e| LatexError {
            message: JsValue::from_str(&e.1.string()),
            location: e.0 as u32,
        })?;
        Ok(LatexToMathML { inner: convert })
    }

    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_local_counter(
        &self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        match self.inner.convert_with_local_counter(
            content,
            if displaystyle {
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

    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_global_counter(
        &mut self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        match self.inner.convert_with_global_counter(
            content,
            if displaystyle {
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

    pub fn reset_global_counter(&mut self) {
        self.inner.reset_global_counter();
    }
}
