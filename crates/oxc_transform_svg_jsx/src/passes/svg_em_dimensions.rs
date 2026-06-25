use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;

use crate::{ExpandProps, Icon, IconSize, TransformError, TransformOptions};

use super::ast::{is_element_named, visit_jsx_openings_mut};
use super::attributes::{AttributeValueSpec, attr_spec, upsert_attribute};

pub(super) fn apply<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    options: &TransformOptions,
) -> Result<(), TransformError> {
    let (width, height) = icon_dimensions(options);
    visit_jsx_openings_mut(root, &mut |element| {
        if is_element_named(element, &["svg", "Svg"]) {
            upsert_attribute(
                allocator,
                element,
                attr_spec("width", width.clone(), ExpandProps::End),
            )?;
            upsert_attribute(
                allocator,
                element,
                attr_spec("height", height.clone(), ExpandProps::End),
            )?;
        }
        Ok(())
    })
}

fn icon_dimensions(options: &TransformOptions) -> (AttributeValueSpec, AttributeValueSpec) {
    match &options.icon {
        Icon::Size(IconSize::Number(value)) => (
            AttributeValueSpec::Number(*value),
            AttributeValueSpec::Number(*value),
        ),
        Icon::Size(IconSize::String(value)) => (
            AttributeValueSpec::String(value.clone()),
            AttributeValueSpec::String(value.clone()),
        ),
        Icon::Default if options.native => (
            AttributeValueSpec::Number(24.0),
            AttributeValueSpec::Number(24.0),
        ),
        _ => (
            AttributeValueSpec::String("1em".into()),
            AttributeValueSpec::String("1em".into()),
        ),
    }
}
