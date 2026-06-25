use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_ast::ast::*;
use oxc_span::{SPAN, Span};

use crate::{ExpandProps, TransformError, attr_name, expression_to_jsx, parse_expression};

#[derive(Debug, Clone)]
pub(super) enum AttributeValueSpec {
    None,
    String(String),
    Number(f64),
    Identifier(String),
    UserExpression(String),
}

#[derive(Debug, Clone)]
pub(super) struct AttributeSpec {
    pub(super) name: String,
    pub(super) value: AttributeValueSpec,
    pub(super) spread: bool,
    pub(super) position: ExpandProps,
}

pub(super) fn attr_spec(
    name: &str,
    value: AttributeValueSpec,
    position: ExpandProps,
) -> AttributeSpec {
    AttributeSpec {
        name: name.into(),
        value,
        spread: false,
        position,
    }
}

pub(super) fn option_attr(name: &str, raw: &str) -> AttributeSpec {
    let value = option_value(raw);
    attr_spec(name, value, ExpandProps::End)
}

pub(super) fn option_value_to_jsx_attr_value<'a>(
    allocator: &'a Allocator,
    raw: &str,
    span: Span,
) -> Result<JSXAttributeValue<'a>, TransformError> {
    attribute_value_to_jsx(allocator, option_value(raw), span)
}

fn option_value(raw: &str) -> AttributeValueSpec {
    if raw.starts_with('{') && raw.ends_with('}') && raw.len() >= 2 {
        AttributeValueSpec::UserExpression(raw[1..raw.len() - 1].to_string())
    } else {
        AttributeValueSpec::String(raw.into())
    }
}

pub(super) fn attribute_value_to_jsx<'a>(
    allocator: &'a Allocator,
    value: AttributeValueSpec,
    span: Span,
) -> Result<JSXAttributeValue<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    Ok(match value {
        AttributeValueSpec::None => {
            return Ok(ast.jsx_attribute_value_string_literal(span, "", None));
        }
        AttributeValueSpec::String(value) => {
            ast.jsx_attribute_value_string_literal(span, ast.str(value.as_str()), None)
        }
        AttributeValueSpec::Number(value) => {
            let expr = ast.expression_numeric_literal(span, value, None, NumberBase::Decimal);
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
        AttributeValueSpec::Identifier(value) => {
            let expr = ast.expression_identifier(span, ast.str(value.as_str()));
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
        AttributeValueSpec::UserExpression(value) => {
            let expr = parse_expression(allocator, value.as_str(), false)?;
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
    })
}

pub(super) fn upsert_attribute<'a>(
    allocator: &'a Allocator,
    element: &mut JSXOpeningElement<'a>,
    spec: AttributeSpec,
) -> Result<(), TransformError> {
    if spec.spread {
        for item in &mut element.attributes {
            let JSXAttributeItem::SpreadAttribute(spread) = item else {
                continue;
            };
            if expression_identifier_name(&spread.argument) == Some(spec.name.as_str()) {
                *item = create_attribute_item(allocator, spec, SPAN)?;
                return Ok(());
            }
        }
    } else {
        for item in &mut element.attributes {
            let JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            if attr_name(&attribute.name) == Some(spec.name.as_str()) {
                *item = create_attribute_item(allocator, spec, attribute.span)?;
                return Ok(());
            }
        }
    }

    let item = create_attribute_item(allocator, spec.clone(), SPAN)?;
    if spec.position == ExpandProps::Start {
        element.attributes.insert(0, item);
    } else {
        element.attributes.push(item);
    }
    Ok(())
}

fn create_attribute_item<'a>(
    allocator: &'a Allocator,
    spec: AttributeSpec,
    span: Span,
) -> Result<JSXAttributeItem<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    if spec.spread {
        let expr = ast.expression_identifier(span, ast.str(spec.name.as_str()));
        return Ok(ast.jsx_attribute_item_spread_attribute(span, expr));
    }
    let value = match spec.value {
        AttributeValueSpec::None => None,
        value => Some(attribute_value_to_jsx(allocator, value, span)?),
    };
    Ok(ast.jsx_attribute_item_attribute(
        span,
        ast.jsx_attribute_name_identifier(span, ast.str(spec.name.as_str())),
        value,
    ))
}

fn expression_identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}
