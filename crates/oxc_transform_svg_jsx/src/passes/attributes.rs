use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_ast::ast::*;
use oxc_span::{SPAN, Span};

use crate::{ExpandProps, TransformError, attr_name, expression_to_jsx, parse_expression};

#[derive(Debug)]
pub(super) enum AttributeValueSpec {
    None,
    String(Cow<'static, str>),
    Number(f64),
    Identifier(Cow<'static, str>),
    UserExpression(Cow<'static, str>),
}

#[derive(Debug)]
pub(super) struct AttributeSpec {
    pub(super) name: Cow<'static, str>,
    pub(super) value: AttributeValueSpec,
    pub(super) spread: bool,
    pub(super) position: ExpandProps,
}

pub(super) fn attr_spec(
    name: &'static str,
    value: AttributeValueSpec,
    position: ExpandProps,
) -> AttributeSpec {
    AttributeSpec {
        name: Cow::Borrowed(name),
        value,
        spread: false,
        position,
    }
}

pub(super) fn option_attr(name: &str, raw: &str) -> AttributeSpec {
    let value = option_value(raw);
    AttributeSpec {
        name: Cow::Owned(name.to_owned()),
        value,
        spread: false,
        position: ExpandProps::End,
    }
}

pub(super) fn option_value_to_jsx_attr_value<'a>(
    allocator: &'a Allocator,
    raw: &str,
    span: Span,
) -> Result<JSXAttributeValue<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    if raw.starts_with('{') && raw.ends_with('}') && raw.len() >= 2 {
        let expr = parse_expression(allocator, &raw[1..raw.len() - 1], false)?;
        Ok(ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr)))
    } else {
        Ok(ast.jsx_attribute_value_string_literal(span, ast.str(raw), None))
    }
}

fn option_value(raw: &str) -> AttributeValueSpec {
    if raw.starts_with('{') && raw.ends_with('}') && raw.len() >= 2 {
        AttributeValueSpec::UserExpression(Cow::Owned(raw[1..raw.len() - 1].to_owned()))
    } else {
        AttributeValueSpec::String(Cow::Owned(raw.to_owned()))
    }
}

pub(super) fn attribute_value_to_jsx<'a>(
    allocator: &'a Allocator,
    value: &AttributeValueSpec,
    span: Span,
) -> Result<JSXAttributeValue<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    Ok(match value {
        AttributeValueSpec::None => {
            return Ok(ast.jsx_attribute_value_string_literal(span, "", None));
        }
        AttributeValueSpec::String(value) => {
            ast.jsx_attribute_value_string_literal(span, ast.str(value.as_ref()), None)
        }
        AttributeValueSpec::Number(value) => {
            let expr = ast.expression_numeric_literal(span, *value, None, NumberBase::Decimal);
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
        AttributeValueSpec::Identifier(value) => {
            let expr = ast.expression_identifier(span, ast.str(value.as_ref()));
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
        AttributeValueSpec::UserExpression(value) => {
            let expr = parse_expression(allocator, value.as_ref(), false)?;
            ast.jsx_attribute_value_expression_container(span, expression_to_jsx(expr))
        }
    })
}

pub(super) fn upsert_attribute<'a>(
    allocator: &'a Allocator,
    element: &mut JSXOpeningElement<'a>,
    spec: &AttributeSpec,
) -> Result<(), TransformError> {
    if spec.spread {
        for item in &mut element.attributes {
            let JSXAttributeItem::SpreadAttribute(spread) = item else {
                continue;
            };
            if expression_identifier_name(&spread.argument) == Some(spec.name.as_ref()) {
                *item = create_attribute_item(allocator, spec, SPAN)?;
                return Ok(());
            }
        }
    } else {
        for item in &mut element.attributes {
            let JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            if attr_name(&attribute.name) == Some(spec.name.as_ref()) {
                *item = create_attribute_item(allocator, spec, attribute.span)?;
                return Ok(());
            }
        }
    }

    let item = create_attribute_item(allocator, spec, SPAN)?;
    if spec.position == ExpandProps::Start {
        element.attributes.insert(0, item);
    } else {
        element.attributes.push(item);
    }
    Ok(())
}

fn create_attribute_item<'a>(
    allocator: &'a Allocator,
    spec: &AttributeSpec,
    span: Span,
) -> Result<JSXAttributeItem<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    if spec.spread {
        let expr = ast.expression_identifier(span, ast.str(spec.name.as_ref()));
        return Ok(ast.jsx_attribute_item_spread_attribute(span, expr));
    }
    let value = match &spec.value {
        AttributeValueSpec::None => None,
        value => Some(attribute_value_to_jsx(allocator, value, span)?),
    };
    Ok(ast.jsx_attribute_item_attribute(
        span,
        ast.jsx_attribute_name_identifier(span, ast.str(spec.name.as_ref())),
        value,
    ))
}

fn expression_identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}
