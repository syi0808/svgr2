use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::{TransformError, TransformOptions};

use super::attributes::option_value_to_jsx_attr_value;
use super::sink::{OpeningElementPass, SinkElementContext};

pub(super) struct ReplaceJsxAttributeValue {
    values: Vec<(String, String)>,
}

impl ReplaceJsxAttributeValue {
    pub(super) fn from_options(options: &TransformOptions) -> Option<Self> {
        if options.replace_attr_values.is_empty() {
            None
        } else {
            Some(Self {
                values: options.replace_attr_values.clone(),
            })
        }
    }
}

impl<'a> OpeningElementPass<'a> for ReplaceJsxAttributeValue {
    fn apply(
        &self,
        allocator: &'a Allocator,
        element: &mut JSXOpeningElement<'a>,
        _context: SinkElementContext,
    ) -> Result<(), TransformError> {
        for item in &mut element.attributes {
            let JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            let Some(replacement) = ({
                let Some(JSXAttributeValue::StringLiteral(current)) = &attribute.value else {
                    continue;
                };
                let current_value = current.value.as_str();
                self.values
                    .iter()
                    .filter(|(old, _)| current_value == old.as_str())
                    .map(|(_, new)| new.as_str())
                    .last()
            }) else {
                continue;
            };
            attribute.value = Some(option_value_to_jsx_attr_value(
                allocator,
                replacement,
                attribute.span,
            )?);
        }
        Ok(())
    }
}
