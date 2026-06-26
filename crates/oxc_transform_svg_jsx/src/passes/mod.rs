mod attributes;
mod post;
mod sink;
mod sink_add_jsx_attribute;
mod sink_remove_jsx_attribute;
mod sink_replace_jsx_attribute_value;
mod sink_svg_em_dimensions;
mod svg_dynamic_title;
mod transform_react_native_svg;

pub(crate) use post::run_post_jsx_passes;
pub(crate) use sink::SinkPasses;
pub(crate) use transform_react_native_svg::collect_native_components;
