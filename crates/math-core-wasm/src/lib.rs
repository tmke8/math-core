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
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    pub message: JsValue,
    pub location: u32,
}

#[wasm_bindgen]
pub struct LatexToMathML {
    inner: math_core::LatexToMathML,
    throw_on_error: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const ITEXT_STYLE: &'static str = r#"
interface MathCoreOptions {
    prettyPrint?: "never" | "always" | "auto";
    macros?: Map<string, string>;
    xmlNamespace?: boolean;
    throwOnError?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "MathCoreOptions")]
    pub type MathCoreOptions;

    #[wasm_bindgen(method, getter)]
    fn prettyPrint(this: &MathCoreOptions) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    fn macros(this: &MathCoreOptions) -> Option<Map>;

    #[wasm_bindgen(method, getter)]
    fn xmlNamespace(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn throwOnError(this: &MathCoreOptions) -> Option<bool>;
}

#[wasm_bindgen]
impl LatexToMathML {
    #[wasm_bindgen(constructor)]
    pub fn new(js_config: &MathCoreOptions) -> Result<LatexToMathML, LatexError> {
        // This is the poor man's `serde_wasm_bindgen::from_value`.
        let macros = js_config.macros().map(|macro_map| {
            let mut macros = Vec::with_capacity(macro_map.size() as usize);
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
                macros.push((key, value));
            }
            macros
        });
        let pretty_print = if let Some(pp) = js_config.prettyPrint() {
            match pp.as_str() {
                "always" => Some(PrettyPrint::Always),
                "never" => Some(PrettyPrint::Never),
                "auto" => Some(PrettyPrint::Auto),
                _ => {
                    return Err(LatexError {
                        message: JsValue::from_str("Invalid value for prettyPrint"),
                        location: 0,
                    });
                }
            }
        } else {
            None
        };
        let xml_namespace = js_config.xmlNamespace();
        let throw_on_error = js_config.throwOnError();
        let config = math_core::MathCoreConfig {
            pretty_print: pretty_print.unwrap_or_default(),
            macros: macros.unwrap_or_default(),
            xml_namespace: xml_namespace.unwrap_or_default(),
        };
        let convert = math_core::LatexToMathML::new(config).map_err(|e| LatexError {
            message: JsValue::from_str(&e.1.string()),
            location: e.0 as u32,
        })?;
        Ok(LatexToMathML {
            inner: convert,
            throw_on_error: throw_on_error.unwrap_or(true),
        })
    }

    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_local_counter(
        &self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self.inner.convert_with_local_counter(content, display) {
            Ok(result) => Ok(JsValue::from_str(&result)),
            Err(mut e) => {
                // Convert the byte offset to a UTF-16 code unit offset for JavaScript.
                e.0 = byte_offset_to_utf16_offset(content, e.0);
                if self.throw_on_error {
                    Err(LatexError {
                        message: JsValue::from_str(&e.1.string()),
                        location: e.0 as u32,
                    })
                } else {
                    Ok(JsValue::from_str(&e.to_html(content, display, None)))
                }
            }
        }
    }

    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_global_counter(
        &mut self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self.inner.convert_with_global_counter(content, display) {
            Ok(result) => Ok(JsValue::from_str(&result)),
            Err(mut e) => {
                // Convert the byte offset to a UTF-16 code unit offset for JavaScript.
                e.0 = byte_offset_to_utf16_offset(content, e.0);
                if self.throw_on_error {
                    Err(LatexError {
                        message: JsValue::from_str(&e.1.string()),
                        location: e.0 as u32,
                    })
                } else {
                    Ok(JsValue::from_str(&e.to_html(content, display, None)))
                }
            }
        }
    }

    pub fn reset_global_counter(&mut self) {
        self.inner.reset_global_counter();
    }
}

/// Converts a byte offset in a UTF-8 string to a UTF-16 code unit offset.
/// This is useful for mapping error locations from Rust (which uses UTF-8) to
/// JavaScript (which uses UTF-16).
///
/// If the byte offset is not a valid character boundary, the original byte
/// offset is returned.
fn byte_offset_to_utf16_offset(s: &str, byte_offset: usize) -> usize {
    s.get(..byte_offset)
        .map(|s| s.chars().map(|c| c.len_utf16()).sum())
        .unwrap_or(byte_offset)
}
