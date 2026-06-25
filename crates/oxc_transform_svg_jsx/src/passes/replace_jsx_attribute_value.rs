use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::TransformError;

use super::ast::visit_jsx_openings_mut;
use super::attributes::option_value_to_jsx_attr_value;

pub(super) fn apply<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    values: &[(String, String)],
) -> Result<(), TransformError> {
    visit_jsx_openings_mut(root, &mut |element| {
        for item in &mut element.attributes {
            let JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            let Some(JSXAttributeValue::StringLiteral(current)) = &attribute.value else {
                continue;
            };
            let current_value = current.value.to_string();
            for (old, new) in values {
                if &current_value == old {
                    attribute.value = Some(option_value_to_jsx_attr_value(
                        allocator,
                        new,
                        attribute.span,
                    )?);
                }
            }
        }
        Ok(())
    })
}
