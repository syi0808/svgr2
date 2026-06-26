use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::{TransformError, TransformOptions, attr_name};

use super::sink::{OpeningElementPass, SinkElementContext, is_svg_element};

pub(super) struct RemoveJsxAttribute {
    names: Vec<&'static str>,
}

impl RemoveJsxAttribute {
    pub(super) fn from_options(options: &TransformOptions) -> Self {
        let mut names = vec!["version"];
        if !options.dimensions {
            names.push("width");
            names.push("height");
        }
        Self { names }
    }
}

impl<'a> OpeningElementPass<'a> for RemoveJsxAttribute {
    fn apply(
        &self,
        _allocator: &'a Allocator,
        element: &mut JSXOpeningElement<'a>,
        context: SinkElementContext,
    ) -> Result<(), TransformError> {
        if context.is_root && is_svg_element(element) {
            element.attributes.retain(|item| {
                let JSXAttributeItem::Attribute(attribute) = item else {
                    return true;
                };
                !attr_name(&attribute.name).is_some_and(|name| self.names.contains(&name))
            });
        }
        Ok(())
    }
}
