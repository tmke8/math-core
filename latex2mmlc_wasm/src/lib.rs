use wasm_bindgen::prelude::*;
use latex2mmlc::{latex_to_mathml, Display};

#[wasm_bindgen]
pub fn convert(content: &str) -> Result<String, JsValue> {
    match latex_to_mathml(content, Display::Inline) {
        Ok(result) => Ok(result),
        Err(e) => {
            Err(JsValue::from_str(&format!("Conversion failed: {:?}", e)))
        }
    }
}

