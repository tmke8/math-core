extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use js_sys::{Array, Map};
use math_core::{LatexError as CoreLatexError, MathDisplay, PrettyPrint};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ConfigParseError {
    message: &'static str,
}

#[wasm_bindgen]
impl ConfigParseError {
    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    pub fn message(&self) -> JsValue {
        JsValue::from_str(&self.message)
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct LatexError {
    message: JsValue,
    pub start: u32,
    pub end: u32,
    context: Option<JsValue>,
    report: Option<JsValue>,
}

#[wasm_bindgen]
impl LatexError {
    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    pub fn message(&self) -> JsValue {
        self.message.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "string | undefined")]
    pub fn context(&self) -> Option<JsValue> {
        self.context.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "string | undefined")]
    pub fn report(&self) -> Option<JsValue> {
        self.report.clone()
    }
}

#[cfg(feature = "ariadne")]
fn error_report(error: &CoreLatexError, source_name: &str, input: &str) -> Option<JsValue> {
    let report = error.to_report(source_name, false);
    let mut buf = Vec::new();
    report
        .write((source_name, ariadne::Source::from(input)), &mut buf)
        .expect("failed to write report");
    Some(JsValue::from_str(
        &String::from_utf8(buf).expect("report should be valid UTF-8"),
    ))
}

#[cfg(not(feature = "ariadne"))]
fn error_report(_error: &CoreLatexError, _source_name: &str, _input: &str) -> Option<JsValue> {
    None
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
    ignoreUnknownCommands?: boolean;
    annotation?: boolean;
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

    #[wasm_bindgen(method, getter)]
    fn ignoreUnknownCommands(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn annotation(this: &MathCoreOptions) -> Option<bool>;
}

#[wasm_bindgen]
impl LatexToMathML {
    #[wasm_bindgen(constructor)]
    pub fn new(js_config: &MathCoreOptions) -> Result<Self, JsValue> {
        // This is the poor man's `serde_wasm_bindgen::from_value`.
        let macros = match js_config.macros() {
            Some(macro_map) => {
                let mut macros = Vec::with_capacity(macro_map.size() as usize);
                let macro_iter = macro_map.entries();
                let success = loop {
                    let Ok(entry) = macro_iter.next() else {
                        break false; // Error getting next entry.
                    };
                    if entry.done() {
                        break true; // Exit the loop when there are no more entries.
                    }
                    let Ok(value) = entry.value().dyn_into::<Array>() else {
                        break false; // Error if the value is not an Array.
                    };
                    let Some(key) = value.get(0).as_string() else {
                        break false; // Error if the first element is not a string.
                    };
                    let Some(value) = value.get(1).as_string() else {
                        break false; // Error if the second element is not a string.
                    };
                    macros.push((key, value));
                };
                if success {
                    Some(macros)
                } else {
                    return Err(ConfigParseError {
                        message: "Invalid macros map",
                    }
                    .into());
                }
            }
            None => None,
        };
        let pretty_print = if let Some(pp) = js_config.prettyPrint() {
            match pp.as_str() {
                "always" => Some(PrettyPrint::Always),
                "never" => Some(PrettyPrint::Never),
                "auto" => Some(PrettyPrint::Auto),
                _ => {
                    return Err(ConfigParseError {
                        message: "Invalid value for prettyPrint",
                    }
                    .into());
                }
            }
        } else {
            None
        };
        let xml_namespace = js_config.xmlNamespace().unwrap_or_default();
        let throw_on_error = js_config.throwOnError();
        let ignore_unknown_commands = js_config.ignoreUnknownCommands().unwrap_or_default();
        let annotation = js_config.annotation().unwrap_or_default();
        let config = math_core::MathCoreConfig {
            pretty_print: pretty_print.unwrap_or_default(),
            macros: macros.unwrap_or_default(),
            xml_namespace,
            ignore_unknown_commands,
            annotation,
        };
        let convert = math_core::LatexToMathML::new(config).map_err(|(e, _, context)| {
            let start = byte_offset_to_utf16_offset(&context, e.0.start) as u32;
            let end = byte_offset_to_utf16_offset(&context, e.0.end) as u32;
            LatexError {
                message: JsValue::from_str(&e.error_message()),
                start,
                end,
                context: Some(JsValue::from_str(&context)),
                report: error_report(&e, "macro", &context),
            }
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
            Err(e) => {
                if self.throw_on_error {
                    // Convert the byte offsets to UTF-16 code unit offsets for JavaScript.
                    let start = byte_offset_to_utf16_offset(content, e.0.start) as u32;
                    let end = byte_offset_to_utf16_offset(content, e.0.end) as u32;
                    Err(LatexError {
                        message: JsValue::from_str(&e.error_message()),
                        start,
                        end,
                        context: None,
                        report: error_report(&e, "input", content),
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
            Err(e) => {
                if self.throw_on_error {
                    // Convert the byte offsets to UTF-16 code unit offsets for JavaScript.
                    let start = byte_offset_to_utf16_offset(content, e.0.start) as u32;
                    let end = byte_offset_to_utf16_offset(content, e.0.end) as u32;
                    Err(LatexError {
                        message: JsValue::from_str(&e.error_message()),
                        start,
                        end,
                        context: None,
                        report: error_report(&e, "input", content),
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
