use std::collections::BTreeSet;

use oxc_ast::ast::*;

use crate::{TransformOptions, element_name};

use super::sink::{ElementNamePass, SinkElementContext};

pub(crate) fn collect_native_components(expression: &Expression<'_>) -> BTreeSet<String> {
    let mut components = BTreeSet::new();
    if let Expression::JSXElement(element) = expression {
        collect_native_components_from_element(element, &mut components);
    }
    components.remove("Svg");
    components
}

pub(super) struct NativeElementNamePass {
    keep_title: bool,
    keep_desc: bool,
}

impl NativeElementNamePass {
    pub(super) fn from_options(options: &TransformOptions) -> Self {
        Self {
            keep_title: options.title_prop,
            keep_desc: options.desc_prop,
        }
    }
}

impl ElementNamePass for NativeElementNamePass {
    fn apply(&self, name: &str, context: SinkElementContext) -> Option<&'static str> {
        if context.is_root && name == "Svg" {
            return Some("Svg");
        }
        if self.keep_title && name == "title" {
            return Some("title");
        }
        if self.keep_desc && name == "desc" {
            return Some("desc");
        }
        native_component_name(name)
    }
}

fn collect_native_components_from_element(
    element: &JSXElement<'_>,
    components: &mut BTreeSet<String>,
) {
    if let Some(name) = element_name(&element.opening_element.name) {
        if name != "Svg" && name.chars().next().is_some_and(char::is_uppercase) {
            components.insert(name.into());
        }
    }
    for child in &element.children {
        if let JSXChild::Element(element) = child {
            collect_native_components_from_element(element, components);
        }
    }
}

pub(super) fn native_component_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "svg" => "Svg",
        "circle" => "Circle",
        "clipPath" => "ClipPath",
        "ellipse" => "Ellipse",
        "g" => "G",
        "linearGradient" => "LinearGradient",
        "radialGradient" => "RadialGradient",
        "line" => "Line",
        "path" => "Path",
        "pattern" => "Pattern",
        "polygon" => "Polygon",
        "polyline" => "Polyline",
        "rect" => "Rect",
        "symbol" => "Symbol",
        "text" => "Text",
        "textPath" => "TextPath",
        "tspan" => "TSpan",
        "use" => "Use",
        "defs" => "Defs",
        "stop" => "Stop",
        "mask" => "Mask",
        "image" => "Image",
        "foreignObject" => "ForeignObject",
        _ => return None,
    })
}
