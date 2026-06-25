use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn transform(source: &str, options_json: Option<String>) -> Result<String, JsValue> {
    oxc_transform_svg_jsx::transform_json(source, options_json.as_deref())
        .map(|result| result.code)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
