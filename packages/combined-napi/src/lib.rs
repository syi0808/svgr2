use std::collections::BTreeMap;

use napi::Error;
use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use oxc_transform_svg_jsx::{
    ExpandProps, ExportType, Icon, IconSize, JsxRuntime, JsxRuntimeImport,
    TransformOptions as OxcTransformOptions,
};
use oxvg_optimiser::{Extends, Jobs};

pub type JsExpandProps = Either<bool, String>;
pub type JsIcon = Either3<bool, f64, String>;
pub type JsStringPairs = Either<Vec<(String, String)>, BTreeMap<String, String>>;

#[napi(object)]
pub struct TransformOptions {
    pub component_name: Option<String>,
    pub previous_export: Option<String>,
    #[napi(js_name = "ref")]
    pub ref_: Option<bool>,
    pub title_prop: Option<bool>,
    pub desc_prop: Option<bool>,
    #[napi(ts_type = "boolean | 'start' | 'end'")]
    pub expand_props: Option<JsExpandProps>,
    pub dimensions: Option<bool>,
    #[napi(ts_type = "boolean | string | number")]
    pub icon: Option<JsIcon>,
    pub native: Option<bool>,
    pub typescript: Option<bool>,
    pub memo: Option<bool>,
    #[napi(ts_type = "Record<string, string> | Array<[string, string]>")]
    pub svg_props: Option<JsStringPairs>,
    #[napi(ts_type = "Record<string, string> | Array<[string, string]>")]
    pub replace_attr_values: Option<JsStringPairs>,
    #[napi(ts_type = "'default' | 'named'")]
    pub export_type: Option<String>,
    pub named_export: Option<String>,
    #[napi(ts_type = "'classic' | 'classic-preact' | 'automatic'")]
    pub jsx_runtime: Option<String>,
    pub jsx_runtime_import: Option<TransformOptionsJsxRuntimeImport>,
    pub import_source: Option<String>,
}

#[napi(object)]
pub struct TransformOptionsJsxRuntimeImport {
    pub source: String,
    pub namespace: Option<String>,
    pub default_specifier: Option<String>,
    pub specifiers: Option<Vec<String>>,
}

impl TransformOptions {
    fn into_oxc_options(self) -> Result<OxcTransformOptions, String> {
        let mut options = OxcTransformOptions::default();

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
            options.expand_props = into_expand_props(value)?;
        }
        if let Some(value) = self.dimensions {
            options.dimensions = value;
        }
        if let Some(value) = self.icon {
            options.icon = into_icon(value);
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
            options.svg_props = into_string_pairs(value);
        }
        if let Some(value) = self.replace_attr_values {
            options.replace_attr_values = into_string_pairs(value);
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

impl TransformOptionsJsxRuntimeImport {
    fn into_jsx_runtime_import(self) -> JsxRuntimeImport {
        JsxRuntimeImport {
            source: self.source,
            namespace: self.namespace,
            default_specifier: self.default_specifier,
            specifiers: self.specifiers.unwrap_or_default(),
        }
    }
}

fn into_expand_props(value: JsExpandProps) -> Result<ExpandProps, String> {
    match value {
        Either::A(true) => Ok(ExpandProps::End),
        Either::A(false) => Ok(ExpandProps::Disabled),
        Either::B(value) => match value.as_str() {
            "start" => Ok(ExpandProps::Start),
            "end" => Ok(ExpandProps::End),
            "false" | "disabled" | "none" => Ok(ExpandProps::Disabled),
            _ => Err(format!("unsupported expandProps value `{value}`")),
        },
    }
}

fn into_icon(value: JsIcon) -> Icon {
    match value {
        Either3::A(true) => Icon::Default,
        Either3::A(false) => Icon::Disabled,
        Either3::B(value) => Icon::Size(IconSize::Number(value)),
        Either3::C(value) => Icon::Size(IconSize::String(value)),
    }
}

fn into_string_pairs(value: JsStringPairs) -> Vec<(String, String)> {
    match value {
        Either::A(values) => values,
        Either::B(values) => values.into_iter().collect(),
    }
}

fn parse_export_type(value: &str) -> Result<ExportType, String> {
    match value {
        "default" => Ok(ExportType::Default),
        "named" => Ok(ExportType::Named),
        _ => Err(format!("unsupported exportType value `{value}`")),
    }
}

fn apply_jsx_runtime(options: &mut OxcTransformOptions, value: &str) -> Result<(), String> {
    match value {
        "classic" => options.jsx_runtime = JsxRuntime::Classic,
        "automatic" => options.jsx_runtime = JsxRuntime::Automatic,
        "classic-preact" => {
            options.jsx_runtime = JsxRuntime::Classic;
            options.import_source = "preact/compat".into();
            options.jsx_runtime_import = Some(JsxRuntimeImport {
                source: "preact".into(),
                namespace: None,
                default_specifier: None,
                specifiers: vec!["h".into()],
            });
        }
        _ => return Err(format!("unsupported jsxRuntime value `{value}`")),
    }
    Ok(())
}

#[napi]
pub fn transform(
    source: String,
    transform_options: Option<TransformOptions>,
    optimise_options: Option<Jobs>,
) -> napi::Result<String> {
    let transform_options = transform_options
        .map_or_else(
            || Ok(OxcTransformOptions::default()),
            TransformOptions::into_oxc_options,
        )
        .map_err(Error::from_reason)?;

    combined::transform(
        &source,
        optimise_options.unwrap_or_default(),
        transform_options,
    )
    .map(|result| result.code)
    .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn extend(extend: Extends, options: Option<Jobs>) -> Jobs {
    options.map_or_else(|| extend.jobs(), |jobs| extend.extend(&jobs))
}
