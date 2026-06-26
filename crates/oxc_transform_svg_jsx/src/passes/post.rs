use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;

use crate::{TransformError, TransformOptions};

use super::svg_dynamic_title;

pub(crate) fn run_post_jsx_passes<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    options: &TransformOptions,
) -> Result<(), TransformError> {
    if options.title_prop {
        svg_dynamic_title::apply(allocator, root, "title")?;
    }
    if options.desc_prop {
        svg_dynamic_title::apply(allocator, root, "desc")?;
    }
    Ok(())
}
