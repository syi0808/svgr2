mod add_jsx_attribute;
mod ast;
mod attributes;
mod remove_jsx_attribute;
mod remove_jsx_empty_expression;
mod replace_jsx_attribute_value;
mod svg_dynamic_title;
mod svg_em_dimensions;
mod transform_react_native_svg;

use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;

use crate::{ExpandProps, Icon, TransformError, TransformOptions};

use attributes::{AttributeSpec, AttributeValueSpec, attr_spec, option_attr};

pub(crate) use transform_react_native_svg::collect_native_components;

const SVG_ELEMENTS: &[&str] = &["svg", "Svg"];

pub(crate) fn run_jsx_passes<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    options: &TransformOptions,
) -> Result<(), TransformError> {
    if options.icon != Icon::Disabled && options.dimensions {
        svg_em_dimensions::apply(allocator, root, options)?;
    }

    let mut to_remove = vec!["version"];
    if !options.dimensions {
        to_remove.push("width");
        to_remove.push("height");
    }
    remove_jsx_attribute::apply(root, SVG_ELEMENTS, &to_remove)?;

    let mut to_add = Vec::new();
    for (name, value) in &options.svg_props {
        to_add.push(option_attr(name, value));
    }
    if options.r#ref {
        to_add.push(attr_spec(
            "ref",
            AttributeValueSpec::Expression("ref".into()),
            ExpandProps::End,
        ));
    }
    if options.title_prop {
        to_add.push(attr_spec(
            "aria-labelledby",
            AttributeValueSpec::Expression("titleId".into()),
            ExpandProps::End,
        ));
    }
    if options.desc_prop {
        to_add.push(attr_spec(
            "aria-describedby",
            AttributeValueSpec::Expression("descId".into()),
            ExpandProps::End,
        ));
    }
    if options.expand_props != ExpandProps::Disabled {
        to_add.push(AttributeSpec {
            name: "props".into(),
            value: AttributeValueSpec::None,
            spread: true,
            position: options.expand_props,
        });
    }
    if !to_add.is_empty() {
        add_jsx_attribute::apply(allocator, root, SVG_ELEMENTS, &to_add)?;
    }

    remove_jsx_empty_expression::apply(root);

    if !options.replace_attr_values.is_empty() {
        replace_jsx_attribute_value::apply(allocator, root, &options.replace_attr_values)?;
    }

    if options.title_prop {
        svg_dynamic_title::apply(allocator, root, "title")?;
    }
    if options.desc_prop {
        svg_dynamic_title::apply(allocator, root, "desc")?;
    }
    if options.native {
        transform_react_native_svg::apply(allocator, root)?;
    }

    Ok(())
}
