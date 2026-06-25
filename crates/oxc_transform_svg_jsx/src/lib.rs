use std::collections::BTreeSet;

use oxc_allocator::{Allocator, CloneIn, TakeIn, Vec as ArenaVec};
use oxc_ast::ast::*;
use oxc_ast::{AstBuilder, NONE};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType, Span};
use svg_parser::{
    Attribute, CData, Comment, EndElement, FinishStartElement, ParseError, ProcessingInstruction,
    Span as SvgSpan, StartElement, SvgSink, Text, parse_with_sink,
};
use thiserror::Error;

mod passes;

use passes::{collect_native_components, run_jsx_passes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformResult {
    pub code: String,
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("{0}")]
    ParseSvg(String),
    #[error("{0}")]
    BuildJsx(String),
    #[error("invalid JavaScript expression `{expr}`")]
    InvalidExpression { expr: String },
    #[error("invalid generated JavaScript: {errors}\n{code}")]
    InvalidGeneratedCode { errors: String, code: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformOptions {
    pub component_name: String,
    pub previous_export: Option<String>,
    pub r#ref: bool,
    pub title_prop: bool,
    pub desc_prop: bool,
    pub expand_props: ExpandProps,
    pub dimensions: bool,
    pub icon: Icon,
    pub native: bool,
    pub typescript: bool,
    pub memo: bool,
    pub svg_props: Vec<(String, String)>,
    pub replace_attr_values: Vec<(String, String)>,
    pub export_type: ExportType,
    pub named_export: String,
    pub jsx_runtime: JsxRuntime,
    pub jsx_runtime_import: Option<JsxRuntimeImport>,
    pub import_source: String,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            component_name: "SvgComponent".into(),
            previous_export: None,
            r#ref: false,
            title_prop: false,
            desc_prop: false,
            expand_props: ExpandProps::End,
            dimensions: true,
            icon: Icon::Disabled,
            native: false,
            typescript: false,
            memo: false,
            svg_props: Vec::new(),
            replace_attr_values: Vec::new(),
            export_type: ExportType::Default,
            named_export: "ReactComponent".into(),
            jsx_runtime: JsxRuntime::Classic,
            jsx_runtime_import: None,
            import_source: "react".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandProps {
    Disabled,
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Icon {
    Disabled,
    Default,
    Size(IconSize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconSize {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportType {
    Default,
    Named,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsxRuntime {
    Classic,
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsxRuntimeImport {
    pub source: String,
    pub namespace: Option<String>,
    pub default_specifier: Option<String>,
    pub specifiers: Vec<String>,
}

pub fn transform(
    source: &str,
    options: TransformOptions,
) -> Result<TransformResult, TransformError> {
    let allocator = Allocator::default();
    let program = build_program(&allocator, source, &options)?;
    let code = Codegen::new()
        .with_options(CodegenOptions {
            minify: false,
            ..CodegenOptions::default()
        })
        .build(&program)
        .code;
    Ok(TransformResult { code })
}

pub fn build_program<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    options: &TransformOptions,
) -> Result<Program<'a>, TransformError> {
    let mut jsx = parse_svg_to_jsx(allocator, source)?;
    run_jsx_passes(allocator, &mut jsx, options)?;
    let final_code = wrap_component(allocator, &jsx, options)?;
    parse_program(allocator, &final_code, options.typescript)
}

fn parse_svg_to_jsx<'a>(
    allocator: &'a Allocator,
    source: &'a str,
) -> Result<Expression<'a>, TransformError> {
    let mut sink = OxcJsxSink::new(allocator);
    parse_with_sink(source, &mut sink).map_err(|error| match error {
        ParseError::Sink(error) => TransformError::BuildJsx(error.to_string()),
        error => TransformError::ParseSvg(error.to_string()),
    })?;
    sink.finish()
}

struct OxcJsxSink<'a> {
    ast: AstBuilder<'a>,
    stack: Vec<ElementFrame<'a>>,
    root: Option<Expression<'a>>,
}

impl<'a> OxcJsxSink<'a> {
    fn new(allocator: &'a Allocator) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            stack: Vec::new(),
            root: None,
        }
    }

    fn finish(self) -> Result<Expression<'a>, TransformError> {
        if !self.stack.is_empty() {
            return Err(TransformError::BuildJsx("unclosed SVG elements".into()));
        }
        self.root.ok_or_else(|| {
            TransformError::BuildJsx("SVG document does not contain a root element".into())
        })
    }

    fn attach_child(&mut self, child: JSXChild<'a>) -> Result<(), SinkError> {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(child);
            Ok(())
        } else {
            match child {
                JSXChild::Element(element) if self.root.is_none() => {
                    self.root = Some(Expression::JSXElement(element));
                    Ok(())
                }
                JSXChild::Element(_) => Err(SinkError::MultipleRoots),
                _ => Ok(()),
            }
        }
    }

    fn build_element(
        &self,
        frame: ElementFrame<'a>,
        closing_span: Option<SvgSpan>,
    ) -> JSXChild<'a> {
        let span = to_span(frame.span);
        let name = map_element_name(&frame.name);
        let opening = self.ast.alloc_jsx_opening_element(
            to_span(frame.opening_span),
            self.ast
                .jsx_element_name_identifier(to_span(frame.name_span), self.ast.str(name.as_str())),
            NONE,
            frame.attributes,
        );
        let closing = if frame.children.is_empty() {
            None
        } else {
            Some(self.ast.alloc_jsx_closing_element(
                closing_span.map_or(span, to_span),
                self.ast.jsx_element_name_identifier(
                    to_span(frame.name_span),
                    self.ast.str(name.as_str()),
                ),
            ))
        };
        JSXChild::Element(
            self.ast
                .alloc_jsx_element(span, opening, frame.children, closing),
        )
    }

    fn text_child(&self, text: Text<'_>) -> JSXChild<'a> {
        let value = decode_xml(text.value);
        let expr = JSXExpression::StringLiteral(self.ast.alloc_string_literal(
            to_span(text.span),
            self.ast.str(value.as_str()),
            None,
        ));
        self.ast
            .jsx_child_expression_container(to_span(text.span), expr)
    }

    fn attr_item(&self, attr: Attribute<'_>, element_name: &str) -> JSXAttributeItem<'a> {
        let mapped_name = map_attribute_name(attr.name, element_name);
        let value = attr.value.map(|value| {
            self.attr_value(
                mapped_name.as_str(),
                value,
                attr.value_span.unwrap_or(attr.span),
            )
        });
        let mapped_name = self.ast.str(mapped_name.as_str());
        self.ast.jsx_attribute_item_attribute(
            to_span(attr.span),
            self.ast
                .jsx_attribute_name_identifier(to_span(attr.name_span), mapped_name),
            value,
        )
    }

    fn attr_value(&self, key: &str, raw: &str, span: SvgSpan) -> JSXAttributeValue<'a> {
        let value = replace_spaces(&decode_xml(raw));
        if key == "style" {
            let expr = style_to_object_expression(self.ast.allocator, value.as_str())
                .unwrap_or_else(|| self.ast.expression_object(SPAN, self.ast.vec()));
            return self
                .ast
                .jsx_attribute_value_expression_container(to_span(span), expression_to_jsx(expr));
        }
        if let Some(number) = numeric_value(value.as_str()) {
            let expr = self.ast.expression_numeric_literal(
                to_span(span),
                number,
                None,
                NumberBase::Decimal,
            );
            return self
                .ast
                .jsx_attribute_value_expression_container(to_span(span), expression_to_jsx(expr));
        }
        self.ast.jsx_attribute_value_string_literal(
            to_span(span),
            self.ast.str(value.as_str()),
            None,
        )
    }
}

#[derive(Debug, Error)]
enum SinkError {
    #[error("unexpected attribute without an element")]
    NoCurrentElement,
    #[error("unexpected closing tag </{0}>")]
    UnexpectedClosingTag(String),
    #[error("expected closing tag </{expected}> but found </{found}>")]
    MismatchedClosingTag { expected: String, found: String },
    #[error("SVG document contains multiple root elements")]
    MultipleRoots,
}

struct ElementFrame<'a> {
    name: String,
    span: SvgSpan,
    opening_span: SvgSpan,
    name_span: SvgSpan,
    attributes: ArenaVec<'a, JSXAttributeItem<'a>>,
    children: ArenaVec<'a, JSXChild<'a>>,
}

impl<'src, 'a> SvgSink<'src> for OxcJsxSink<'a> {
    type Error = SinkError;

    fn start_element(&mut self, event: StartElement<'src>) -> Result<(), Self::Error> {
        self.stack.push(ElementFrame {
            name: event.name.to_string(),
            span: event.span,
            opening_span: event.span,
            name_span: event.name_span,
            attributes: self.ast.vec(),
            children: self.ast.vec(),
        });
        Ok(())
    }

    fn attribute(&mut self, attr: Attribute<'src>) -> Result<(), Self::Error> {
        let Some(current) = self.stack.last() else {
            return Err(SinkError::NoCurrentElement);
        };
        let element_name = current.name.clone();
        let item = self.attr_item(attr, &element_name);
        self.stack
            .last_mut()
            .ok_or(SinkError::NoCurrentElement)?
            .attributes
            .push(item);
        Ok(())
    }

    fn finish_start_element(&mut self, event: FinishStartElement) -> Result<(), Self::Error> {
        if event.self_closing {
            let Some(mut frame) = self.stack.pop() else {
                return Err(SinkError::NoCurrentElement);
            };
            frame.span.end = event.span.end;
            frame.opening_span = event.span;
            let child = self.build_element(frame, None);
            self.attach_child(child)?;
        } else if let Some(current) = self.stack.last_mut() {
            current.opening_span = event.span;
            current.span.end = event.span.end;
        }
        Ok(())
    }

    fn end_element(&mut self, event: EndElement<'src>) -> Result<(), Self::Error> {
        let Some(mut frame) = self.stack.pop() else {
            return Err(SinkError::UnexpectedClosingTag(event.name.into()));
        };
        if frame.name != event.name {
            return Err(SinkError::MismatchedClosingTag {
                expected: frame.name,
                found: event.name.into(),
            });
        }
        frame.span.end = event.span.end;
        let child = self.build_element(frame, Some(event.span));
        self.attach_child(child)?;
        Ok(())
    }

    fn text(&mut self, text: Text<'src>) -> Result<(), Self::Error> {
        let child = self.text_child(text);
        self.attach_child(child)
    }

    fn comment(&mut self, _comment: Comment<'src>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn cdata(&mut self, cdata: CData<'src>) -> Result<(), Self::Error> {
        self.text(Text {
            value: cdata.value,
            span: cdata.span,
        })
    }

    fn processing_instruction(
        &mut self,
        _instruction: ProcessingInstruction<'src>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn wrap_component<'a>(
    allocator: &'a Allocator,
    jsx: &Expression<'a>,
    options: &TransformOptions,
) -> Result<String, TransformError> {
    let jsx_code = codegen_expression(allocator, jsx, options.typescript);
    let jsx_code = trim_expression_statement(&jsx_code);
    let native_components = if options.native {
        collect_native_components(jsx)
    } else {
        BTreeSet::new()
    };

    let mut code = String::new();
    write_imports(&mut code, options, &native_components)?;
    if options.typescript && (options.title_prop || options.desc_prop) {
        code.push_str(&svgr_props_interface(options));
    }
    let params = component_params(options);
    code.push_str("const ");
    code.push_str(&options.component_name);
    code.push_str(" = ");
    code.push_str(&params);
    code.push_str(" => ");
    code.push_str(jsx_code);
    code.push_str(";\n");

    let mut export_identifier = options.component_name.clone();
    if options.r#ref {
        code.push_str("const ForwardRef = forwardRef(");
        code.push_str(&export_identifier);
        code.push_str(");\n");
        export_identifier = "ForwardRef".into();
    }
    if options.memo {
        code.push_str("const Memo = memo(");
        code.push_str(&export_identifier);
        code.push_str(");\n");
        export_identifier = "Memo".into();
    }

    if options.previous_export.is_some() || options.export_type == ExportType::Named {
        code.push_str("export { ");
        code.push_str(&export_identifier);
        code.push_str(" as ");
        code.push_str(&options.named_export);
        code.push_str(" };\n");
        if let Some(previous_export) = &options.previous_export {
            code.push_str(previous_export.trim());
            code.push('\n');
        }
    } else {
        code.push_str("export default ");
        code.push_str(&export_identifier);
        code.push_str(";\n");
    }

    Ok(code)
}

fn parse_program<'a>(
    allocator: &'a Allocator,
    source: &str,
    typescript: bool,
) -> Result<Program<'a>, TransformError> {
    let source = allocator.alloc_str(source);
    let source_type = source_type(typescript);
    let ret = Parser::new(allocator, source, source_type).parse();
    if ret.panicked || !ret.errors.is_empty() {
        let errors = ret
            .errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(TransformError::InvalidGeneratedCode {
            errors,
            code: source.to_string(),
        });
    }
    Ok(ret.program)
}

pub(crate) fn parse_expression<'a>(
    allocator: &'a Allocator,
    source: &str,
    typescript: bool,
) -> Result<Expression<'a>, TransformError> {
    let wrapped = format!("const __svgr_expr = ({source});");
    let mut program = parse_program(allocator, &wrapped, typescript).map_err(|_| {
        TransformError::InvalidExpression {
            expr: source.into(),
        }
    })?;
    let Some(statement) = program.body.pop() else {
        return Err(TransformError::InvalidExpression {
            expr: source.into(),
        });
    };
    let Statement::VariableDeclaration(mut decl) = statement else {
        return Err(TransformError::InvalidExpression {
            expr: source.into(),
        });
    };
    let declarations = decl.declarations.take_in(AstBuilder::new(allocator));
    let Some(mut declarator) = declarations.into_iter().next() else {
        return Err(TransformError::InvalidExpression {
            expr: source.into(),
        });
    };
    declarator
        .init
        .take()
        .ok_or_else(|| TransformError::InvalidExpression {
            expr: source.into(),
        })
}

pub(crate) fn parse_jsx_expression<'a>(
    allocator: &'a Allocator,
    source: &str,
) -> Result<JSXExpression<'a>, TransformError> {
    parse_expression(allocator, source, false).map(expression_to_jsx)
}

pub(crate) fn expression_to_jsx<'a>(expression: Expression<'a>) -> JSXExpression<'a> {
    match expression {
        Expression::BooleanLiteral(it) => JSXExpression::BooleanLiteral(it),
        Expression::NullLiteral(it) => JSXExpression::NullLiteral(it),
        Expression::NumericLiteral(it) => JSXExpression::NumericLiteral(it),
        Expression::BigIntLiteral(it) => JSXExpression::BigIntLiteral(it),
        Expression::RegExpLiteral(it) => JSXExpression::RegExpLiteral(it),
        Expression::StringLiteral(it) => JSXExpression::StringLiteral(it),
        Expression::TemplateLiteral(it) => JSXExpression::TemplateLiteral(it),
        Expression::Identifier(it) => JSXExpression::Identifier(it),
        Expression::MetaProperty(it) => JSXExpression::MetaProperty(it),
        Expression::Super(it) => JSXExpression::Super(it),
        Expression::ArrayExpression(it) => JSXExpression::ArrayExpression(it),
        Expression::ArrowFunctionExpression(it) => JSXExpression::ArrowFunctionExpression(it),
        Expression::AssignmentExpression(it) => JSXExpression::AssignmentExpression(it),
        Expression::AwaitExpression(it) => JSXExpression::AwaitExpression(it),
        Expression::BinaryExpression(it) => JSXExpression::BinaryExpression(it),
        Expression::CallExpression(it) => JSXExpression::CallExpression(it),
        Expression::ChainExpression(it) => JSXExpression::ChainExpression(it),
        Expression::ClassExpression(it) => JSXExpression::ClassExpression(it),
        Expression::ConditionalExpression(it) => JSXExpression::ConditionalExpression(it),
        Expression::FunctionExpression(it) => JSXExpression::FunctionExpression(it),
        Expression::ImportExpression(it) => JSXExpression::ImportExpression(it),
        Expression::LogicalExpression(it) => JSXExpression::LogicalExpression(it),
        Expression::NewExpression(it) => JSXExpression::NewExpression(it),
        Expression::ObjectExpression(it) => JSXExpression::ObjectExpression(it),
        Expression::ParenthesizedExpression(it) => JSXExpression::ParenthesizedExpression(it),
        Expression::SequenceExpression(it) => JSXExpression::SequenceExpression(it),
        Expression::TaggedTemplateExpression(it) => JSXExpression::TaggedTemplateExpression(it),
        Expression::ThisExpression(it) => JSXExpression::ThisExpression(it),
        Expression::UnaryExpression(it) => JSXExpression::UnaryExpression(it),
        Expression::UpdateExpression(it) => JSXExpression::UpdateExpression(it),
        Expression::YieldExpression(it) => JSXExpression::YieldExpression(it),
        Expression::PrivateInExpression(it) => JSXExpression::PrivateInExpression(it),
        Expression::JSXElement(it) => JSXExpression::JSXElement(it),
        Expression::JSXFragment(it) => JSXExpression::JSXFragment(it),
        Expression::TSAsExpression(it) => JSXExpression::TSAsExpression(it),
        Expression::TSSatisfiesExpression(it) => JSXExpression::TSSatisfiesExpression(it),
        Expression::TSTypeAssertion(it) => JSXExpression::TSTypeAssertion(it),
        Expression::TSNonNullExpression(it) => JSXExpression::TSNonNullExpression(it),
        Expression::TSInstantiationExpression(it) => JSXExpression::TSInstantiationExpression(it),
        Expression::V8IntrinsicExpression(it) => JSXExpression::V8IntrinsicExpression(it),
        Expression::ComputedMemberExpression(it) => JSXExpression::ComputedMemberExpression(it),
        Expression::StaticMemberExpression(it) => JSXExpression::StaticMemberExpression(it),
        Expression::PrivateFieldExpression(it) => JSXExpression::PrivateFieldExpression(it),
    }
}

fn source_type(typescript: bool) -> SourceType {
    SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(typescript)
}

fn codegen_expression<'a>(
    allocator: &'a Allocator,
    expression: &Expression<'a>,
    typescript: bool,
) -> String {
    let ast = AstBuilder::new(allocator);
    let mut body = ast.vec();
    body.push(ast.statement_expression(SPAN, expression.clone_in(allocator)));
    let program = ast.program(
        SPAN,
        source_type(typescript),
        "",
        ast.vec(),
        None,
        ast.vec(),
        body,
    );
    Codegen::new().build(&program).code
}

pub(crate) fn codegen_jsx_element<'a>(
    allocator: &'a Allocator,
    element: &JSXElement<'a>,
    typescript: bool,
) -> String {
    let ast = AstBuilder::new(allocator);
    let expression = Expression::JSXElement(ast.alloc(element.clone_in(allocator)));
    trim_expression_statement(&codegen_expression(allocator, &expression, typescript)).to_string()
}

fn trim_expression_statement(code: &str) -> &str {
    code.trim().trim_end_matches(';').trim()
}

pub(crate) fn element_name<'a>(name: &'a JSXElementName<'a>) -> Option<&'a str> {
    match name {
        JSXElementName::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn attr_name<'a>(name: &'a JSXAttributeName<'a>) -> Option<&'a str> {
    match name {
        JSXAttributeName::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn write_imports(
    code: &mut String,
    options: &TransformOptions,
    native_components: &BTreeSet<String>,
) -> Result<(), TransformError> {
    if options.jsx_runtime != JsxRuntime::Automatic {
        let import = options
            .jsx_runtime_import
            .clone()
            .unwrap_or_else(default_jsx_runtime_import);
        if let Some(namespace) = import.namespace {
            code.push_str("import * as ");
            code.push_str(&namespace);
            code.push_str(" from ");
            code.push_str(&js_string(&import.source));
            code.push_str(";\n");
        } else if let Some(default_specifier) = import.default_specifier {
            code.push_str("import ");
            code.push_str(&default_specifier);
            code.push_str(" from ");
            code.push_str(&js_string(&import.source));
            code.push_str(";\n");
        } else if !import.specifiers.is_empty() {
            code.push_str("import { ");
            code.push_str(&import.specifiers.join(", "));
            code.push_str(" } from ");
            code.push_str(&js_string(&import.source));
            code.push_str(";\n");
        } else {
            return Err(TransformError::BuildJsx(
                "jsxRuntimeImport requires namespace, defaultSpecifier, or specifiers".into(),
            ));
        }
    }

    if options.native {
        code.push_str("import Svg");
        if !native_components.is_empty() {
            code.push_str(", { ");
            code.push_str(
                &native_components
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            code.push_str(" }");
        }
        code.push_str(" from \"react-native-svg\";\n");
    }

    let mut named_react_imports = Vec::new();
    if options.r#ref {
        named_react_imports.push("forwardRef");
    }
    if options.memo {
        named_react_imports.push("memo");
    }
    if !named_react_imports.is_empty() {
        code.push_str("import { ");
        code.push_str(&named_react_imports.join(", "));
        code.push_str(" } from ");
        code.push_str(&js_string(&options.import_source));
        code.push_str(";\n");
    }
    if options.typescript && (options.expand_props != ExpandProps::Disabled || options.r#ref) {
        let mut type_imports = Vec::new();
        if options.expand_props != ExpandProps::Disabled {
            type_imports.push(if options.native {
                "SvgProps"
            } else {
                "SVGProps"
            });
        }
        if options.r#ref && !options.native {
            type_imports.push("Ref");
        }
        if !type_imports.is_empty() {
            code.push_str("import type { ");
            code.push_str(&type_imports.join(", "));
            code.push_str(" } from ");
            code.push_str(&js_string(if options.native {
                "react-native-svg"
            } else {
                &options.import_source
            }));
            code.push_str(";\n");
        }
    }
    Ok(())
}

fn component_params(options: &TransformOptions) -> String {
    let mut props = Vec::new();
    if options.title_prop {
        props.push("title");
        props.push("titleId");
    }
    if options.desc_prop {
        props.push("desc");
        props.push("descId");
    }
    let props_type = props_type(options);
    let first = if props.is_empty() {
        if options.expand_props == ExpandProps::Disabled {
            if options.r#ref {
                "_".into()
            } else {
                return "()".into();
            }
        } else {
            format!("props{props_type}")
        }
    } else {
        let mut pattern = String::from("{ ");
        pattern.push_str(&props.join(", "));
        if options.expand_props != ExpandProps::Disabled {
            pattern.push_str(", ...props");
        }
        pattern.push_str(" }");
        pattern.push_str(&props_type);
        pattern
    };
    if options.r#ref {
        let ref_type = if options.typescript && !options.native {
            ": Ref<SVGSVGElement>"
        } else {
            ""
        };
        format!("({first}, ref{ref_type})")
    } else if options.typescript || first.starts_with('{') {
        format!("({first})")
    } else {
        first
    }
}

fn props_type(options: &TransformOptions) -> String {
    if !options.typescript {
        return String::new();
    }
    let svg_props = if options.native {
        "SvgProps".to_string()
    } else {
        "SVGProps<SVGSVGElement>".to_string()
    };
    match (
        options.expand_props != ExpandProps::Disabled,
        options.title_prop || options.desc_prop,
    ) {
        (true, true) => format!(": {svg_props} & SVGRProps"),
        (true, false) => format!(": {svg_props}"),
        (false, true) => ": SVGRProps".into(),
        (false, false) => String::new(),
    }
}

fn svgr_props_interface(options: &TransformOptions) -> String {
    let mut code = String::from("interface SVGRProps {\n");
    if options.title_prop {
        code.push_str("title?: string;\ntitleId?: string;\n");
    }
    if options.desc_prop {
        code.push_str("desc?: string;\ndescId?: string;\n");
    }
    code.push_str("}\n");
    code
}

fn default_jsx_runtime_import() -> JsxRuntimeImport {
    JsxRuntimeImport {
        source: "react".into(),
        namespace: Some("React".into()),
        default_specifier: None,
        specifiers: Vec::new(),
    }
}

fn map_element_name(name: &str) -> String {
    match name {
        "clippath" => "clipPath".into(),
        "lineargradient" => "linearGradient".into(),
        "radialgradient" => "radialGradient".into(),
        "textpath" => "textPath".into(),
        "foreignobject" => "foreignObject".into(),
        "feblend" => "feBlend".into(),
        "fecolormatrix" => "feColorMatrix".into(),
        "fecomponenttransfer" => "feComponentTransfer".into(),
        "fecomposite" => "feComposite".into(),
        "feconvolvematrix" => "feConvolveMatrix".into(),
        "fediffuselighting" => "feDiffuseLighting".into(),
        "fedisplacementmap" => "feDisplacementMap".into(),
        "fedistantlight" => "feDistantLight".into(),
        "fedropshadow" => "feDropShadow".into(),
        "feflood" => "feFlood".into(),
        "fefunca" => "feFuncA".into(),
        "fefuncb" => "feFuncB".into(),
        "fefuncg" => "feFuncG".into(),
        "fefuncr" => "feFuncR".into(),
        "fegaussianblur" => "feGaussianBlur".into(),
        "feimage" => "feImage".into(),
        "femerge" => "feMerge".into(),
        "femergenode" => "feMergeNode".into(),
        "femorphology" => "feMorphology".into(),
        "feoffset" => "feOffset".into(),
        "fepointlight" => "fePointLight".into(),
        "fespecularlighting" => "feSpecularLighting".into(),
        "fespotlight" => "feSpotLight".into(),
        "fetile" => "feTile".into(),
        "feturbulence" => "feTurbulence".into(),
        _ => name.into(),
    }
}

fn map_attribute_name(name: &str, element_name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if element_name == "input" && lower == "checked" {
        return "defaultChecked".into();
    }
    if element_name == "input" && lower == "value" {
        return "defaultValue".into();
    }
    match lower.as_str() {
        "class" | "classname" => "className".into(),
        "for" => "htmlFor".into(),
        "tabindex" => "tabIndex".into(),
        "viewbox" => "viewBox".into(),
        "preserveaspectratio" => "preserveAspectRatio".into(),
        "xmlns:xlink" | "xmlnsxlink" => "xmlnsXlink".into(),
        "xml:space" | "xmlspace" => "xmlSpace".into(),
        "xml:lang" | "xmllang" => "xmlLang".into(),
        "xlink:href" | "xlinkhref" => "xlinkHref".into(),
        "xlink:title" | "xlinktitle" => "xlinkTitle".into(),
        "xlink:type" | "xlinktype" => "xlinkType".into(),
        _ if lower.starts_with("aria-") => convert_aria_attribute(&lower),
        _ if lower.starts_with("data-") => lower,
        _ => camelize_svg_name(name),
    }
}

fn camelize_svg_name(name: &str) -> String {
    let mut result = String::new();
    let mut upper_next = false;
    for ch in name.chars() {
        if ch == '-' || ch == ':' {
            upper_next = true;
        } else if upper_next {
            result.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn convert_aria_attribute(name: &str) -> String {
    let mut parts = name.split('-');
    let first = parts.next().unwrap_or_default();
    let rest = parts.collect::<String>().to_ascii_lowercase();
    format!("{first}-{rest}")
}

fn style_to_object_expression<'a>(allocator: &'a Allocator, raw: &str) -> Option<Expression<'a>> {
    let mut props = Vec::new();
    for entry in raw.split(';') {
        let style = entry.trim();
        if style.is_empty() {
            continue;
        }
        let Some((key, value)) = style.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let formatted_key = if key.starts_with("--") {
            js_string(key)
        } else {
            camelize_svg_name(key.trim_start_matches("-ms-"))
        };
        let value = value.trim();
        let formatted_value =
            if let Some(px) = value.strip_suffix("px").and_then(|v| numeric_value(v)) {
                number_literal(px)
            } else if let Some(number) = numeric_value(value) {
                number_literal(number)
            } else {
                js_string(value)
            };
        if key.starts_with("--") {
            props.push(format!("{formatted_key}: {formatted_value}"));
        } else {
            props.push(format!("{formatted_key}: {formatted_value}"));
        }
    }
    parse_expression(allocator, &format!("{{ {} }}", props.join(", ")), false).ok()
}

fn numeric_value(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let number = trimmed.parse::<f64>().ok()?;
    if number.is_finite() {
        Some(number)
    } else {
        None
    }
}

fn number_literal(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

fn replace_spaces(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\t' | '\r' | '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}' => ' ',
            ch => ch,
        })
        .collect()
}

fn decode_xml(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        result.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        let Some(end) = rest.find(';') else {
            result.push('&');
            result.push_str(rest);
            return result;
        };
        let entity = &rest[..end];
        rest = &rest[end + 1..];
        match entity {
            "amp" => result.push('&'),
            "lt" => result.push('<'),
            "gt" => result.push('>'),
            "quot" => result.push('"'),
            "apos" => result.push('\''),
            _ if entity.starts_with("#x") => {
                if let Ok(value) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(ch) = char::from_u32(value) {
                        result.push(ch);
                    }
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(value) = entity[1..].parse::<u32>() {
                    if let Some(ch) = char::from_u32(value) {
                        result.push(ch);
                    }
                }
            }
            _ => {
                result.push('&');
                result.push_str(entity);
                result.push(';');
            }
        }
    }
    result.push_str(rest);
    result
}

pub(crate) fn js_string(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            ch => result.push(ch),
        }
    }
    result.push('"');
    result
}

fn to_span(span: SvgSpan) -> Span {
    Span::new(span.start as u32, span.end as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(svg: &str, options: TransformOptions) -> String {
        transform(svg, options).unwrap().code
    }

    #[test]
    fn transforms_simple_svg() {
        let result = code(
            r#"<svg width="88px" height="88px" viewBox="0 0 88 88" version="1.1"><title>Dismiss</title><g stroke-width="2"><path d="M0 0" /></g></svg>"#,
            TransformOptions::default(),
        );
        assert!(result.contains("import * as React from \"react\";"));
        assert!(result.contains("const SvgComponent"));
        assert!(result.contains("<svg width=\"88px\" height=\"88px\" viewBox=\"0 0 88 88\""));
        assert!(result.contains("<title>{\"Dismiss\"}</title>"));
        assert!(result.contains("strokeWidth={2}"));
        assert!(result.contains("{...props}"));
        assert!(result.contains("export default SvgComponent;"));
    }

    #[test]
    fn supports_automatic_runtime_and_dimensions_false() {
        let options = TransformOptions {
            jsx_runtime: JsxRuntime::Automatic,
            dimensions: false,
            ..TransformOptions::default()
        };
        let result = code(
            r#"<svg width="10" height="20" viewBox="0 0 10 20" />"#,
            options,
        );
        assert!(!result.contains("import * as React"));
        assert!(result.contains("<svg viewBox=\"0 0 10 20\" {...props} />"));
    }

    #[test]
    fn supports_icon_and_replace_values() {
        let options = TransformOptions {
            icon: Icon::Size(IconSize::Number(24.0)),
            replace_attr_values: vec![("#fff".into(), "{props.color}".into())],
            ..TransformOptions::default()
        };
        let result = code(r##"<svg fill="#fff" />"##, options);
        assert!(result.contains("width={24}"));
        assert!(result.contains("height={24}"));
        assert!(result.contains("fill={props.color}"));
    }

    #[test]
    fn supports_title_and_desc_props() {
        let options = TransformOptions {
            title_prop: true,
            desc_prop: true,
            ..TransformOptions::default()
        };
        let result = code(
            r#"<svg><title id="a">Hello</title><desc>World</desc></svg>"#,
            options,
        );
        assert!(result.contains("({ title, titleId, desc, descId, ...props })"));
        assert!(result.contains("aria-labelledby={titleId}"));
        assert!(result.contains("aria-describedby={descId}"));
        assert!(result.contains("title === undefined ? <title id={titleId || \"a\"}>"));
        assert!(result.contains("desc === undefined ? <desc id={descId}>"));
    }

    #[test]
    fn supports_native() {
        let options = TransformOptions {
            native: true,
            icon: Icon::Default,
            ..TransformOptions::default()
        };
        let result = code(r#"<svg><g><path d="M0 0" /></g><div /></svg>"#, options);
        assert!(result.contains("import Svg, { G, Path } from \"react-native-svg\";"));
        assert!(
            result.contains(
                "<Svg width={24} height={24} {...props}><G><Path d=\"M0 0\" /></G></Svg>"
            )
        );
        assert!(!result.contains("div"));
    }

    #[test]
    fn supports_ref_memo_named_export_and_previous_export() {
        let options = TransformOptions {
            r#ref: true,
            memo: true,
            export_type: ExportType::Named,
            named_export: "Component".into(),
            previous_export: Some("export default \"logo.svg\";".into()),
            ..TransformOptions::default()
        };
        let result = code(r#"<svg />"#, options);
        assert!(result.contains("import { forwardRef, memo } from \"react\";"));
        assert!(result.contains("const ForwardRef = forwardRef(SvgComponent);"));
        assert!(result.contains("const Memo = memo(ForwardRef);"));
        assert!(result.contains("export { Memo as Component };"));
        assert!(result.contains("export default \"logo.svg\";"));
    }

    #[test]
    fn supports_typescript_props() {
        let options = TransformOptions {
            typescript: true,
            title_prop: true,
            r#ref: true,
            ..TransformOptions::default()
        };
        let result = code(r#"<svg />"#, options);
        assert!(result.contains("interface SVGRProps"));
        assert!(result.contains("import type { SVGProps, Ref } from \"react\";"));
        assert!(result.contains("SVGProps<SVGSVGElement> & SVGRProps"));
        assert!(result.contains("ref: Ref<SVGSVGElement>"));
    }
}
