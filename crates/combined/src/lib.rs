use oxc_transform_svg_jsx::{TransformOptions, TransformResult};
use oxvg_ast::{
    parse::roxmltree::parse,
    serialize::{Node as _, Options as SerializeOptions},
    visitor::Info,
};
use oxvg_optimiser::Jobs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CombinedError {
    #[error("unable to parse SVG for optimisation: {0}")]
    ParseOxvg(String),
    #[error("unable to optimise SVG: {0}")]
    Optimise(String),
    #[error("unable to serialize optimised SVG: {0}")]
    Serialize(String),
    #[error(transparent)]
    Transform(#[from] oxc_transform_svg_jsx::TransformError),
}

pub fn transform(
    source: &str,
    optimise_options: Jobs,
    transform_options: TransformOptions,
) -> Result<TransformResult, CombinedError> {
    let optimised = parse(source, |dom, allocator| {
        optimise_options
            .run(dom, &Info::new(allocator))
            .map_err(|error| CombinedError::Optimise(error.to_string()))?;

        dom.serialize_with_options(SerializeOptions::default())
            .map_err(|error| CombinedError::Serialize(error.to_string()))
    })
    .map_err(|error| CombinedError::ParseOxvg(error.to_string()))??;

    oxc_transform_svg_jsx::transform(&optimised, transform_options).map_err(CombinedError::from)
}

#[cfg(test)]
mod tests {
    use oxc_transform_svg_jsx::TransformOptions;
    use oxvg_optimiser::Jobs;

    use super::transform;

    #[test]
    fn optimises_then_transforms_in_one_call() {
        let result = transform(
            r#"<svg viewBox="0 0 10 10"><!-- comment --><path d="M 0 0 L 10 10"/></svg>"#,
            Jobs::default(),
            TransformOptions {
                component_name: "Icon".into(),
                ..TransformOptions::default()
            },
        )
        .unwrap()
        .code;

        assert!(result.contains("const Icon ="));
        assert!(result.contains("<path"));
        assert!(!result.contains("comment"));
    }
}
