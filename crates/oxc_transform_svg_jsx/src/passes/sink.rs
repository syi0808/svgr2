use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use crate::{TransformError, TransformOptions, element_name, map_element_name};

use super::sink_add_jsx_attribute::AddJsxAttribute;
use super::sink_remove_jsx_attribute::RemoveJsxAttribute;
use super::sink_replace_jsx_attribute_value::ReplaceJsxAttributeValue;
use super::sink_svg_em_dimensions::SvgEmDimensions;
use super::transform_react_native_svg::NativeElementNamePass;

const SVG_ELEMENTS: &[&str] = &["svg", "Svg"];

#[derive(Debug, Clone, Copy)]
pub(crate) struct SinkElementContext {
    pub(crate) is_root: bool,
}

pub(crate) trait ElementNamePass {
    fn apply(&self, name: String, context: SinkElementContext) -> Option<String>;
}

pub(crate) trait OpeningElementPass<'a> {
    fn apply(
        &self,
        allocator: &'a Allocator,
        element: &mut JSXOpeningElement<'a>,
        context: SinkElementContext,
    ) -> Result<(), TransformError>;
}

pub(crate) struct SinkPasses<'a> {
    allocator: &'a Allocator,
    element_name_passes: Vec<Box<dyn ElementNamePass + 'a>>,
    opening_element_passes: Vec<Box<dyn OpeningElementPass<'a> + 'a>>,
}

impl<'a> SinkPasses<'a> {
    pub(crate) fn from_options(allocator: &'a Allocator, options: &TransformOptions) -> Self {
        let mut element_name_passes: Vec<Box<dyn ElementNamePass + 'a>> = Vec::new();
        let mut opening_element_passes: Vec<Box<dyn OpeningElementPass<'a> + 'a>> = Vec::new();

        if let Some(pass) = SvgEmDimensions::from_options(options) {
            opening_element_passes.push(Box::new(pass));
        }
        opening_element_passes.push(Box::new(RemoveJsxAttribute::from_options(options)));
        if let Some(pass) = AddJsxAttribute::from_options(options) {
            opening_element_passes.push(Box::new(pass));
        }
        if let Some(pass) = ReplaceJsxAttributeValue::from_options(options) {
            opening_element_passes.push(Box::new(pass));
        }

        if options.native {
            element_name_passes.push(Box::new(NativeElementNamePass::from_options(options)));
        }

        Self {
            allocator,
            element_name_passes,
            opening_element_passes,
        }
    }

    pub(crate) fn prepare_element_name(&self, svg_name: &str, is_root: bool) -> Option<String> {
        let context = SinkElementContext { is_root };
        let mut name = map_element_name(svg_name);
        for pass in &self.element_name_passes {
            name = pass.apply(name, context)?;
        }
        Some(name)
    }

    pub(crate) fn apply_opening_element(
        &self,
        element: &mut JSXOpeningElement<'a>,
        is_root: bool,
    ) -> Result<(), TransformError> {
        let context = SinkElementContext { is_root };
        for pass in &self.opening_element_passes {
            pass.apply(self.allocator, element, context)?;
        }
        Ok(())
    }
}

pub(super) fn is_svg_element(element: &JSXOpeningElement<'_>) -> bool {
    element_name(&element.name).is_some_and(|name| SVG_ELEMENTS.contains(&name))
}
