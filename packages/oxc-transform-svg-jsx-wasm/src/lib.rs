use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn transform(source: &str, options_json: Option<String>) -> Result<String, JsValue> {
    let options = oxc_transform_svg_jsx_options::parse_options_json(options_json.as_deref())
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    oxc_transform_svg_jsx::transform(source, options)
        .map(|result| result.code)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
