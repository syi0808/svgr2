use oxc_ast::ast::*;

use crate::{TransformError, attr_name};

use super::ast::{is_element_named, visit_jsx_openings_mut};

pub(super) fn apply(
    root: &mut Expression<'_>,
    elements: &[&str],
    attributes: &[&str],
) -> Result<(), TransformError> {
    visit_jsx_openings_mut(root, &mut |element| {
        if is_element_named(element, elements) {
            remove_attributes(element, attributes);
        }
        Ok(())
    })
}

fn remove_attributes(element: &mut JSXOpeningElement<'_>, names: &[&str]) {
    element.attributes.retain(|item| {
        let JSXAttributeItem::Attribute(attribute) = item else {
            return true;
        };
        !attr_name(&attribute.name).is_some_and(|name| names.contains(&name))
    });
}
