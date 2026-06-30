use oxvg_optimiser::Jobs;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn transform(
    source: &str,
    transform_options_json: Option<String>,
    optimise_options_json: Option<String>,
) -> Result<String, JsValue> {
    let transform_options =
        oxc_transform_svg_jsx_options::parse_options_json(transform_options_json.as_deref())
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let optimise_options = parse_optimise_options(optimise_options_json.as_deref())?;

    combined::transform(source, optimise_options, transform_options)
        .map(|result| result.code)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

fn parse_optimise_options(options_json: Option<&str>) -> Result<Jobs, JsValue> {
    match options_json {
        Some(raw) if !raw.trim().is_empty() => {
            serde_json::from_str(raw).map_err(|error| JsValue::from_str(&error.to_string()))
        }
        _ => Ok(Jobs::default()),
    }
}
