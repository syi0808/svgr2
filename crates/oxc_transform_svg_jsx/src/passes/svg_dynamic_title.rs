use oxc_allocator::{Allocator, Box as ArenaBox, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::{AstBuilder, NONE};
use oxc_span::SPAN;

use crate::{ExpandProps, TransformError, attr_name, element_name, expression_to_jsx};

use super::attributes::{AttributeValueSpec, attr_spec, upsert_attribute};

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
        svg.children.insert(
            0,
            AstBuilder::new(allocator).jsx_child_expression_container(
                SPAN,
                expression_to_jsx(dynamic_tag_expression(allocator, tag, None)),
            ),
        );
        return Ok(());
    };

    let fallback = match &mut svg.children[index] {
        JSXChild::Element(element) => {
            ensure_dynamic_tag_id(allocator, element, tag)?;
            if element.children.is_empty() {
                None
            } else {
                Some(element.clone_in(allocator))
            }
        }
        _ => None,
    };

    svg.children[index] = AstBuilder::new(allocator).jsx_child_expression_container(
        SPAN,
        expression_to_jsx(dynamic_tag_expression(allocator, tag, fallback)),
    );
    Ok(())
}

fn dynamic_tag_expression<'a>(
    allocator: &'a Allocator,
    tag: &str,
    fallback: Option<ArenaBox<'a, JSXElement<'a>>>,
) -> Expression<'a> {
    let ast = AstBuilder::new(allocator);
    let tag_expr = ast.expression_identifier(SPAN, ast.str(tag));
    let conditional = ast.expression_conditional(
        SPAN,
        tag_expr,
        Expression::JSXElement(create_dynamic_tag_element(allocator, tag)),
        ast.expression_null_literal(SPAN),
    );
    if let Some(fallback) = fallback {
        let ast = AstBuilder::new(allocator);
        ast.expression_conditional(
            SPAN,
            ast.expression_binary(
                SPAN,
                ast.expression_identifier(SPAN, ast.str(tag)),
                BinaryOperator::StrictEquality,
                ast.expression_identifier(SPAN, ast.str("undefined")),
            ),
            Expression::JSXElement(fallback),
            conditional,
        )
    } else {
        conditional
    }
}

fn create_dynamic_tag_element<'a>(
    allocator: &'a Allocator,
    tag: &str,
) -> ArenaBox<'a, JSXElement<'a>> {
    let ast = AstBuilder::new(allocator);
    let mut attributes = ast.vec();
    attributes.push(create_tag_id_attribute(allocator, tag));
    let mut children = ast.vec();
    children.push(ast.jsx_child_expression_container(
        SPAN,
        expression_to_jsx(ast.expression_identifier(SPAN, ast.str(tag))),
    ));
    let tag_name = ast.str(tag);
    ast.alloc_jsx_element(
        SPAN,
        ast.alloc_jsx_opening_element(
            SPAN,
            ast.jsx_element_name_identifier(SPAN, tag_name),
            NONE,
            attributes,
        ),
        children,
        Some(ast.alloc_jsx_closing_element(SPAN, ast.jsx_element_name_identifier(SPAN, tag_name))),
    )
}

fn create_tag_id_attribute<'a>(allocator: &'a Allocator, tag: &str) -> JSXAttributeItem<'a> {
    let ast = AstBuilder::new(allocator);
    let id_name = dynamic_tag_id_name(tag);
    ast.jsx_attribute_item_attribute(
        SPAN,
        ast.jsx_attribute_name_identifier(SPAN, ast.str("id")),
        Some(ast.jsx_attribute_value_expression_container(
            SPAN,
            expression_to_jsx(ast.expression_identifier(SPAN, ast.str(id_name))),
        )),
    )
}

fn dynamic_tag_id_name(tag: &str) -> &'static str {
    match tag {
        "title" => "titleId",
        "desc" => "descId",
        _ => unreachable!("unsupported dynamic SVG metadata tag"),
    }
}

fn ensure_element_has_closing<'a>(allocator: &'a Allocator, element: &mut JSXElement<'a>) {
    if element.closing_element.is_some() {
        return;
    }
    let ast = AstBuilder::new(allocator);
    let closing = {
        let Some(name) = element_name(&element.opening_element.name) else {
            return;
        };
        let name = ast.str(name);
        ast.alloc_jsx_closing_element(
            element.opening_element.span,
            ast.jsx_element_name_identifier(element.opening_element.span, name),
        )
    };
    element.closing_element = Some(closing);
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
        let ast = AstBuilder::new(allocator);
        let id_name = dynamic_tag_id_name(tag);
        let expr = match &attribute.value {
            Some(JSXAttributeValue::StringLiteral(lit)) => ast.expression_logical(
                attribute.span,
                ast.expression_identifier(attribute.span, ast.str(id_name)),
                LogicalOperator::Or,
                ast.expression_string_literal(attribute.span, ast.str(lit.value.as_str()), None),
            ),
            _ => ast.expression_identifier(attribute.span, ast.str(id_name)),
        };
        attribute.value = Some(
            ast.jsx_attribute_value_expression_container(attribute.span, expression_to_jsx(expr)),
        );
        return Ok(());
    }
    let spec = attr_spec(
        "id",
        AttributeValueSpec::Identifier(dynamic_tag_id_name(tag).into()),
        ExpandProps::End,
    );
    upsert_attribute(allocator, &mut element.opening_element, &spec)
}
