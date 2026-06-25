use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{
    ExpandProps, ExportType, Icon, IconSize, JsxRuntime, JsxRuntimeImport, TransformError,
    TransformOptions, TransformResult, transform,
};

pub fn transform_json(
    source: &str,
    options_json: Option<&str>,
) -> Result<TransformResult, TransformError> {
    let options = match options_json {
        Some(raw) if !raw.trim().is_empty() => {
            let json_options: JsonTransformOptions = serde_json::from_str(raw)
                .map_err(|error| TransformError::InvalidOptions(error.to_string()))?;
            json_options.into_transform_options()?
        }
        _ => TransformOptions::default(),
    };
    transform(source, options)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonTransformOptions {
    component_name: Option<String>,
    previous_export: Option<String>,
    #[serde(rename = "ref")]
    ref_: Option<bool>,
    title_prop: Option<bool>,
    desc_prop: Option<bool>,
    expand_props: Option<JsonExpandProps>,
    dimensions: Option<bool>,
    icon: Option<JsonIcon>,
    native: Option<bool>,
    typescript: Option<bool>,
    memo: Option<bool>,
    svg_props: Option<JsonStringPairs>,
    replace_attr_values: Option<JsonStringPairs>,
    export_type: Option<String>,
    named_export: Option<String>,
    jsx_runtime: Option<String>,
    jsx_runtime_import: Option<JsonJsxRuntimeImport>,
    import_source: Option<String>,
}

impl JsonTransformOptions {
    fn into_transform_options(self) -> Result<TransformOptions, TransformError> {
        let mut options = TransformOptions::default();

        if let Some(value) = self.component_name {
            options.component_name = value;
        }
        if let Some(value) = self.previous_export {
            options.previous_export = Some(value);
        }
        if let Some(value) = self.ref_ {
            options.r#ref = value;
        }
        if let Some(value) = self.title_prop {
            options.title_prop = value;
        }
        if let Some(value) = self.desc_prop {
            options.desc_prop = value;
        }
        if let Some(value) = self.expand_props {
            options.expand_props = value.into_expand_props()?;
        }
        if let Some(value) = self.dimensions {
            options.dimensions = value;
        }
        if let Some(value) = self.icon {
            options.icon = value.into_icon()?;
        }
        if let Some(value) = self.native {
            options.native = value;
        }
        if let Some(value) = self.typescript {
            options.typescript = value;
        }
        if let Some(value) = self.memo {
            options.memo = value;
        }
        if let Some(value) = self.svg_props {
            options.svg_props = value.into_vec();
        }
        if let Some(value) = self.replace_attr_values {
            options.replace_attr_values = value.into_vec();
        }
        if let Some(value) = self.export_type {
            options.export_type = parse_export_type(&value)?;
        }
        if let Some(value) = self.named_export {
            options.named_export = value;
        }
        if let Some(value) = self.import_source {
            options.import_source = value;
        }
        if let Some(value) = self.jsx_runtime {
            apply_jsx_runtime(&mut options, &value)?;
        }
        if let Some(value) = self.jsx_runtime_import {
            let import = value.into_jsx_runtime_import();
            options.import_source = import.source.clone();
            options.jsx_runtime = JsxRuntime::Classic;
            options.jsx_runtime_import = Some(import);
        }

        Ok(options)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonExpandProps {
    Bool(bool),
    String(String),
}

impl JsonExpandProps {
    fn into_expand_props(self) -> Result<ExpandProps, TransformError> {
        match self {
            Self::Bool(true) => Ok(ExpandProps::End),
            Self::Bool(false) => Ok(ExpandProps::Disabled),
            Self::String(value) => match value.as_str() {
                "start" => Ok(ExpandProps::Start),
                "end" => Ok(ExpandProps::End),
                "false" | "disabled" | "none" => Ok(ExpandProps::Disabled),
                _ => Err(invalid_option(format!(
                    "unsupported expandProps value `{value}`"
                ))),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonIcon {
    Bool(bool),
    Number(f64),
    String(String),
}

impl JsonIcon {
    fn into_icon(self) -> Result<Icon, TransformError> {
        match self {
            Self::Bool(true) => Ok(Icon::Default),
            Self::Bool(false) => Ok(Icon::Disabled),
            Self::Number(value) => Ok(Icon::Size(IconSize::Number(value))),
            Self::String(value) => Ok(Icon::Size(IconSize::String(value))),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonStringPairs {
    Map(BTreeMap<String, String>),
    Pairs(Vec<(String, String)>),
}

impl JsonStringPairs {
    fn into_vec(self) -> Vec<(String, String)> {
        match self {
            Self::Map(values) => values.into_iter().collect(),
            Self::Pairs(values) => values,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonJsxRuntimeImport {
    source: String,
    namespace: Option<String>,
    default_specifier: Option<String>,
    #[serde(default)]
    specifiers: Vec<String>,
}

impl JsonJsxRuntimeImport {
    fn into_jsx_runtime_import(self) -> JsxRuntimeImport {
        JsxRuntimeImport {
            source: self.source,
            namespace: self.namespace,
            default_specifier: self.default_specifier,
            specifiers: self.specifiers,
        }
    }
}

fn parse_export_type(value: &str) -> Result<ExportType, TransformError> {
    match value {
        "default" => Ok(ExportType::Default),
        "named" => Ok(ExportType::Named),
        _ => Err(invalid_option(format!(
            "unsupported exportType value `{value}`"
        ))),
    }
}

fn apply_jsx_runtime(options: &mut TransformOptions, value: &str) -> Result<(), TransformError> {
    match value {
        "classic" => {
            options.jsx_runtime = JsxRuntime::Classic;
            Ok(())
        }
        "automatic" => {
            options.jsx_runtime = JsxRuntime::Automatic;
            Ok(())
        }
        "classic-preact" => {
            options.jsx_runtime = JsxRuntime::Classic;
            options.import_source = "preact/compat".into();
            options.jsx_runtime_import = Some(JsxRuntimeImport {
                source: "preact".into(),
                namespace: None,
                default_specifier: None,
                specifiers: vec!["h".into()],
            });
            Ok(())
        }
        _ => Err(invalid_option(format!(
            "unsupported jsxRuntime value `{value}`"
        ))),
    }
}

fn invalid_option(message: String) -> TransformError {
    TransformError::InvalidOptions(message)
}
