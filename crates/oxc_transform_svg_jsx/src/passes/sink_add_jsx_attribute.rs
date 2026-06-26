use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::{ExpandProps, TransformError, TransformOptions};

use super::attributes::{
    AttributeSpec, AttributeValueSpec, attr_spec, option_attr, upsert_attribute,
};
use super::sink::{OpeningElementPass, SinkElementContext, is_svg_element};

pub(super) struct AddJsxAttribute {
    attributes: Vec<AttributeSpec>,
}

impl AddJsxAttribute {
    pub(super) fn from_options(options: &TransformOptions) -> Option<Self> {
        let mut attributes = Vec::new();
        for (name, value) in &options.svg_props {
            attributes.push(option_attr(name, value));
        }
        if options.r#ref {
            attributes.push(attr_spec(
                "ref",
                AttributeValueSpec::Identifier("ref".into()),
                ExpandProps::End,
            ));
        }
        if options.title_prop {
            attributes.push(attr_spec(
                "aria-labelledby",
                AttributeValueSpec::Identifier("titleId".into()),
                ExpandProps::End,
            ));
        }
        if options.desc_prop {
            attributes.push(attr_spec(
                "aria-describedby",
                AttributeValueSpec::Identifier("descId".into()),
                ExpandProps::End,
            ));
        }
        if options.expand_props != ExpandProps::Disabled {
            attributes.push(AttributeSpec {
                name: "props".into(),
                value: AttributeValueSpec::None,
                spread: true,
                position: options.expand_props,
            });
        }
        if attributes.is_empty() {
            None
        } else {
            Some(Self { attributes })
        }
    }
}

impl<'a> OpeningElementPass<'a> for AddJsxAttribute {
    fn apply(
        &self,
        allocator: &'a Allocator,
        element: &mut JSXOpeningElement<'a>,
        context: SinkElementContext,
    ) -> Result<(), TransformError> {
        if !context.is_root || !is_svg_element(element) {
            return Ok(());
        }
        for spec in &self.attributes {
            upsert_attribute(allocator, element, spec)?;
        }
        Ok(())
    }
}
