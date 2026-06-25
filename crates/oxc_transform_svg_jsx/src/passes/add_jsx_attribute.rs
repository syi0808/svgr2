use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;

use crate::TransformError;

use super::ast::{is_element_named, visit_jsx_openings_mut};
use super::attributes::{AttributeSpec, upsert_attribute};

pub(super) fn apply<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    elements: &[&str],
    attributes: &[AttributeSpec],
) -> Result<(), TransformError> {
    visit_jsx_openings_mut(root, &mut |element| {
        if is_element_named(element, elements) {
            for spec in attributes {
                upsert_attribute(allocator, element, spec.clone())?;
            }
        }
        Ok(())
    })
}
