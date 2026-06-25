use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_ast::ast::*;
use oxc_span::SPAN;

use crate::{
    ExpandProps, TransformError, attr_name, codegen_jsx_element, element_name, js_string,
    parse_jsx_expression,
};

use super::attributes::{AttributeValueSpec, attr_spec, attribute_value_to_jsx, upsert_attribute};

pub(super) fn apply<'a>(
    allocator: &'a Allocator,
    root: &mut Expression<'a>,
    tag: &'static str,
) -> Result<(), TransformError> {
    let Expression::JSXElement(svg) = root else {
        return Ok(());
    };
    if !matches!(element_name(&svg.opening_element.name), Some("svg" | "Svg")) {
        return Ok(());
    }
    ensure_element_has_closing(allocator, svg);

    let Some(index) = svg.children.iter().position(|child| {
        matches!(
            child,
            JSXChild::Element(element)
                if matches!(element_name(&element.opening_element.name), Some(name) if name == tag)
        )
    }) else {
        let expression = format!("{tag} ? <{tag} id={{{tag}Id}}>{{{tag}}}</{tag}> : null");
        svg.children.insert(
            0,
            AstBuilder::new(allocator).jsx_child_expression_container(
                SPAN,
                parse_jsx_expression(allocator, &expression)?,
            ),
        );
        return Ok(());
    };

    let existing_has_children = match &mut svg.children[index] {
        JSXChild::Element(element) => {
            ensure_dynamic_tag_id(allocator, element, tag)?;
            !element.children.is_empty()
        }
        _ => false,
    };

    let expression = if existing_has_children {
        let fallback = match &svg.children[index] {
            JSXChild::Element(element) => codegen_jsx_element(allocator, element, false),
            _ => unreachable!(),
        };
        format!(
            "{tag} === undefined ? {fallback} : {tag} ? <{tag} id={{{tag}Id}}>{{{tag}}}</{tag}> : null"
        )
    } else {
        format!("{tag} ? <{tag} id={{{tag}Id}}>{{{tag}}}</{tag}> : null")
    };
    svg.children[index] = AstBuilder::new(allocator)
        .jsx_child_expression_container(SPAN, parse_jsx_expression(allocator, &expression)?);
    Ok(())
}

fn ensure_element_has_closing<'a>(allocator: &'a Allocator, element: &mut JSXElement<'a>) {
    if element.closing_element.is_some() {
        return;
    }
    let Some(name) = element_name(&element.opening_element.name).map(ToOwned::to_owned) else {
        return;
    };
    let ast = AstBuilder::new(allocator);
    element.closing_element = Some(ast.alloc_jsx_closing_element(
        element.opening_element.span,
        ast.jsx_element_name_identifier(element.opening_element.span, ast.str(name.as_str())),
    ));
}

fn ensure_dynamic_tag_id<'a>(
    allocator: &'a Allocator,
    element: &mut JSXElement<'a>,
    tag: &str,
) -> Result<(), TransformError> {
    for item in &mut element.opening_element.attributes {
        let JSXAttributeItem::Attribute(attribute) = item else {
            continue;
        };
        if attr_name(&attribute.name) != Some("id") {
            continue;
        }
        let expr = match &attribute.value {
            Some(JSXAttributeValue::StringLiteral(lit)) => {
                format!("{tag}Id || {}", js_string(lit.value.as_str()))
            }
            _ => format!("{tag}Id"),
        };
        attribute.value = Some(attribute_value_to_jsx(
            allocator,
            AttributeValueSpec::Expression(expr),
            attribute.span,
        )?);
        return Ok(());
    }
    upsert_attribute(
        allocator,
        &mut element.opening_element,
        attr_spec(
            "id",
            AttributeValueSpec::Expression(format!("{tag}Id")),
            ExpandProps::End,
        ),
    )
}
