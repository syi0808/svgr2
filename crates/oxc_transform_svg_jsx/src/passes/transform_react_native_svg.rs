use std::collections::BTreeSet;

use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::AstBuilder;
use oxc_ast::ast::*;

use crate::{TransformError, element_name};

pub(crate) fn collect_native_components(expression: &Expression<'_>) -> BTreeSet<String> {
    let mut components = BTreeSet::new();
    if let Expression::JSXElement(element) = expression {
        collect_native_components_from_element(element, &mut components);
    }
    components.remove("Svg");
    components
}

pub(super) fn apply<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
) -> Result<(), TransformError> {
    if let Expression::JSXElement(element) = root {
        transform_native_element(allocator, element)?;
    }
    Ok(())
}

fn transform_native_element<'a>(
    allocator: &'a Allocator,
    element: &mut JSXElement<'a>,
) -> Result<bool, TransformError> {
    let ast = AstBuilder::new(allocator);
    let Some(name) = element_name(&element.opening_element.name).map(ToOwned::to_owned) else {
        return Ok(true);
    };
    let Some(component) = native_component_name(name.as_str()) else {
        return Ok(false);
    };
    element.opening_element.name =
        ast.jsx_element_name_identifier(element.opening_element.span, component);
    if let Some(closing) = &mut element.closing_element {
        closing.name = ast.jsx_element_name_identifier(closing.span, component);
    }

    let mut next = ast.vec();
    let children = element.children.take_in(ast);
    for mut child in children {
        let keep = if let JSXChild::Element(child_element) = &mut child {
            transform_native_element(allocator, child_element)?
        } else {
            true
        };
        if keep {
            next.push(child);
        }
    }
    element.children = next;
    Ok(true)
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

fn native_component_name(name: &str) -> Option<&'static str> {
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
