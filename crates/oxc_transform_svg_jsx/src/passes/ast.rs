use oxc_ast::ast::*;

use crate::{TransformError, element_name};

pub(super) fn visit_jsx_openings_mut<'a, F>(
    expression: &mut Expression<'a>,
    visitor: &mut F,
) -> Result<(), TransformError>
where
    F: FnMut(&mut JSXOpeningElement<'a>) -> Result<(), TransformError>,
{
    if let Expression::JSXElement(element) = expression {
        visit_jsx_element_mut(element, visitor)?;
    }
    Ok(())
}

fn visit_jsx_element_mut<'a, F>(
    element: &mut JSXElement<'a>,
    visitor: &mut F,
) -> Result<(), TransformError>
where
    F: FnMut(&mut JSXOpeningElement<'a>) -> Result<(), TransformError>,
{
    visitor(&mut element.opening_element)?;
    for child in &mut element.children {
        if let JSXChild::Element(child_element) = child {
            visit_jsx_element_mut(child_element, visitor)?;
        }
    }
    Ok(())
}

pub(super) fn is_element_named(element: &JSXOpeningElement<'_>, names: &[&str]) -> bool {
    element_name(&element.name).is_some_and(|name| names.contains(&name))
}
