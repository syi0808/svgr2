use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::{ExpandProps, Icon, IconSize, TransformError, TransformOptions};

use super::attributes::{AttributeValueSpec, attr_spec, upsert_attribute};
use super::sink::{OpeningElementPass, SinkElementContext, is_svg_element};

pub(super) struct SvgEmDimensions {
    width: AttributeValueSpec,
    height: AttributeValueSpec,
}

impl SvgEmDimensions {
    pub(super) fn from_options(options: &TransformOptions) -> Option<Self> {
        if options.icon == Icon::Disabled || !options.dimensions {
            return None;
        }
        let (width, height) = icon_dimensions(options);
        Some(Self { width, height })
    }
}

impl<'a> OpeningElementPass<'a> for SvgEmDimensions {
    fn apply(
        &self,
        allocator: &'a Allocator,
        element: &mut JSXOpeningElement<'a>,
        context: SinkElementContext,
    ) -> Result<(), TransformError> {
        if !context.is_root || !is_svg_element(element) {
            return Ok(());
        }
        upsert_attribute(
            allocator,
            element,
            attr_spec("width", self.width.clone(), ExpandProps::End),
        )?;
        upsert_attribute(
            allocator,
            element,
            attr_spec("height", self.height.clone(), ExpandProps::End),
        )
    }
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
