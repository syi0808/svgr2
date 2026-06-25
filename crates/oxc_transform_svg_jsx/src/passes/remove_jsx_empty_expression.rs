use oxc_ast::ast::*;

pub(super) fn apply(expression: &mut Expression<'_>) {
    if let Expression::JSXElement(element) = expression {
        remove_empty_expressions_from_element(element);
    }
}

fn remove_empty_expressions_from_element(element: &mut JSXElement<'_>) {
    element.children.retain(|child| {
        !matches!(
            child,
            JSXChild::ExpressionContainer(container)
                if matches!(container.expression, JSXExpression::EmptyExpression(_))
        )
    });
    for child in &mut element.children {
        if let JSXChild::Element(element) = child {
            remove_empty_expressions_from_element(element);
        }
    }
}
