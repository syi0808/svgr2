use std::collections::BTreeMap;

use oxc_transform_svg_jsx::{
    ExpandProps, ExportType, Icon, IconSize, JsxRuntime, JsxRuntimeImport, TransformError,
    TransformOptions,
};
use serde::Deserialize;

pub fn parse_options_json(options_json: Option<&str>) -> Result<TransformOptions, TransformError> {
    match options_json {
        Some(raw) if !raw.trim().is_empty() => {
            let json_options: JsonTransformOptions = serde_json::from_str(raw)
                .map_err(|error| TransformError::InvalidOptions(error.to_string()))?;
            json_options.into_transform_options()
        }
        _ => Ok(TransformOptions::default()),
    }
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
            options.icon = value.into_icon();
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
    fn into_icon(self) -> Icon {
        match self {
            Self::Bool(true) => Icon::Default,
            Self::Bool(false) => Icon::Disabled,
            Self::Number(value) => Icon::Size(IconSize::Number(value)),
            Self::String(value) => Icon::Size(IconSize::String(value)),
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

#[cfg(test)]
mod tests {
    use oxc_transform_svg_jsx::{
        ExpandProps, ExportType, Icon, IconSize, JsxRuntime, JsxRuntimeImport, TransformError,
        TransformOptions, transform,
    };

    use super::parse_options_json;

    #[test]
    fn defaults_empty_options() {
        assert_eq!(
            parse_options_json(None).unwrap(),
            TransformOptions::default()
        );
        assert_eq!(
            parse_options_json(Some(" \n\t")).unwrap(),
            TransformOptions::default()
        );
    }

    #[test]
    fn parses_all_option_shapes() {
        let options = parse_options_json(Some(
            r##"{
                "componentName": "Icon",
                "previousExport": "export default 'icon.svg'",
                "ref": true,
                "titleProp": true,
                "descProp": true,
                "expandProps": false,
                "dimensions": false,
                "icon": 24,
                "native": true,
                "typescript": true,
                "memo": true,
                "svgProps": [["role", "img"]],
                "replaceAttrValues": {"#000": "{props.color}"},
                "exportType": "named",
                "namedExport": "Component",
                "jsxRuntime": "automatic",
                "jsxRuntimeImport": {
                    "source": "custom",
                    "namespace": "JSX",
                    "defaultSpecifier": "createElement",
                    "specifiers": ["Fragment"]
                },
                "importSource": "ignored"
            }"##,
        ))
        .unwrap();

        assert_eq!(options.component_name, "Icon");
        assert_eq!(
            options.previous_export.as_deref(),
            Some("export default 'icon.svg'")
        );
        assert!(options.r#ref);
        assert!(options.title_prop);
        assert!(options.desc_prop);
        assert_eq!(options.expand_props, ExpandProps::Disabled);
        assert!(!options.dimensions);
        assert_eq!(options.icon, Icon::Size(IconSize::Number(24.0)));
        assert!(options.native);
        assert!(options.typescript);
        assert!(options.memo);
        assert_eq!(options.svg_props, vec![("role".into(), "img".into())]);
        assert_eq!(
            options.replace_attr_values,
            vec![("#000".into(), "{props.color}".into())]
        );
        assert_eq!(options.export_type, ExportType::Named);
        assert_eq!(options.named_export, "Component");
        assert_eq!(options.jsx_runtime, JsxRuntime::Classic);
        assert_eq!(options.import_source, "custom");
        assert_eq!(
            options.jsx_runtime_import,
            Some(JsxRuntimeImport {
                source: "custom".into(),
                namespace: Some("JSX".into()),
                default_specifier: Some("createElement".into()),
                specifiers: vec!["Fragment".into()],
            })
        );
    }

    #[test]
    fn rejects_malformed_json_and_unsupported_values() {
        assert!(matches!(
            parse_options_json(Some("{")),
            Err(TransformError::InvalidOptions(_))
        ));
        assert!(matches!(
            parse_options_json(Some(r#"{"expandProps":"middle"}"#)),
            Err(TransformError::InvalidOptions(_))
        ));
        assert!(matches!(
            parse_options_json(Some(r#"{"exportType":"commonjs"}"#)),
            Err(TransformError::InvalidOptions(_))
        ));
        assert!(matches!(
            parse_options_json(Some(r#"{"jsxRuntime":"custom"}"#)),
            Err(TransformError::InvalidOptions(_))
        ));
    }

    #[test]
    fn supports_json_options_for_wrappers() {
        let options = parse_options_json(Some(
            r##"{
              "componentName": "Icon",
              "jsxRuntime": "classic-preact",
              "dimensions": false,
              "svgProps": { "role": "img" },
              "replaceAttrValues": { "#fff": "{props.color}" },
              "expandProps": "start"
            }"##,
        ))
        .unwrap();
        let result = transform(r##"<svg width="10" height="10" fill="#fff" />"##, options)
            .unwrap()
            .code;

        insta::assert_snapshot!(result);
    }
}
