use napi::Error;
use napi_derive::napi;

#[napi]
pub fn transform(source: String, options_json: Option<String>) -> napi::Result<String> {
    oxc_transform_svg_jsx::transform_json(&source, options_json.as_deref())
        .map(|result| result.code)
        .map_err(|error| Error::from_reason(error.to_string()))
}
