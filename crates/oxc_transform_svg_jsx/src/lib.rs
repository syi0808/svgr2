use std::borrow::Cow;
use std::collections::BTreeSet;

use oxc_allocator::{Allocator, TakeIn, Vec as ArenaVec};
use oxc_ast::ast::*;
use oxc_ast::{AstBuilder, NONE};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType, Span};
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::keyword::is_reserved_keyword;
use svg_parser::{
    Attribute, CData, Comment, EndElement, FinishStartElement, ParseError, ProcessingInstruction,
    Span as SvgSpan, StartElement, SvgSink, Text, parse_with_sink,
};
use thiserror::Error;

mod passes;

use passes::{SinkPasses, collect_native_components, run_post_jsx_passes};

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
    #[error("invalid transform options: {0}")]
    InvalidOptions(String),
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
    validate_options(options)?;
    let mut jsx = parse_svg_to_jsx(allocator, source, options)?;
    run_post_jsx_passes(allocator, &mut jsx, options)?;
    build_component_program(allocator, jsx, options)
}

fn parse_svg_to_jsx<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    options: &TransformOptions,
) -> Result<Expression<'a>, TransformError> {
    let mut sink = OxcJsxSink::new(allocator, options);
    parse_with_sink(source, &mut sink).map_err(|error| match error {
        ParseError::Sink(SinkError::Transform(error)) => error,
        ParseError::Sink(error) => TransformError::BuildJsx(error.to_string()),
        error => TransformError::ParseSvg(error.to_string()),
    })?;
    sink.finish()
}

struct OxcJsxSink<'src, 'a> {
    ast: AstBuilder<'a>,
    passes: SinkPasses<'a>,
    stack: Vec<ElementFrame<'src, 'a>>,
    root: Option<Expression<'a>>,
}

impl<'src, 'a> OxcJsxSink<'src, 'a> {
    fn new(allocator: &'a Allocator, options: &TransformOptions) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            passes: SinkPasses::from_options(allocator, options),
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
        frame: ElementFrame<'src, 'a>,
        closing_span: Option<SvgSpan>,
        is_root: bool,
    ) -> Result<Option<JSXChild<'a>>, TransformError> {
        let Some(name) = self.passes.prepare_element_name(&frame.name, is_root) else {
            return Ok(None);
        };
        let span = to_span(frame.span);
        let mut opening = self.ast.alloc_jsx_opening_element(
            to_span(frame.opening_span),
            self.ast
                .jsx_element_name_identifier(to_span(frame.name_span), self.ast.str(name.as_ref())),
            NONE,
            frame.attributes,
        );
        self.passes.apply_opening_element(&mut opening, is_root)?;
        let name = element_name(&opening.name).unwrap_or(name.as_ref());
        let closing = if frame.children.is_empty() {
            None
        } else {
            Some(
                self.ast.alloc_jsx_closing_element(
                    closing_span.map_or(span, to_span),
                    self.ast
                        .jsx_element_name_identifier(to_span(frame.name_span), self.ast.str(name)),
                ),
            )
        };
        Ok(Some(JSXChild::Element(self.ast.alloc_jsx_element(
            span,
            opening,
            frame.children,
            closing,
        ))))
    }

    fn text_child(&self, text: Text<'src>) -> JSXChild<'a> {
        let value = decode_xml(text.value);
        let expr = JSXExpression::StringLiteral(self.ast.alloc_string_literal(
            to_span(text.span),
            self.ast.str(value.as_ref()),
            None,
        ));
        self.ast
            .jsx_child_expression_container(to_span(text.span), expr)
    }

    fn attr_item(&self, attr: Attribute<'src>, element_name: &str) -> JSXAttributeItem<'a> {
        let mapped_name = map_attribute_name(attr.name, element_name);
        let value = attr.value.map(|value| {
            self.attr_value(
                mapped_name.as_ref(),
                value,
                attr.value_span.unwrap_or(attr.span),
            )
        });
        let mapped_name = self.ast.str(mapped_name.as_ref());
        self.ast.jsx_attribute_item_attribute(
            to_span(attr.span),
            self.ast
                .jsx_attribute_name_identifier(to_span(attr.name_span), mapped_name),
            value,
        )
    }

    fn attr_value(&self, key: &str, raw: &str, span: SvgSpan) -> JSXAttributeValue<'a> {
        let value = normalize_attribute_value(raw);
        if key == "style" {
            let expr = style_to_object_expression(self.ast.allocator, value.as_ref())
                .unwrap_or_else(|| self.ast.expression_object(SPAN, self.ast.vec()));
            return self
                .ast
                .jsx_attribute_value_expression_container(to_span(span), expression_to_jsx(expr));
        }
        if !is_always_string_attribute(key)
            && let Some(number) = numeric_value(value.as_ref())
        {
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
            self.ast.str(value.as_ref()),
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
    #[error(transparent)]
    Transform(#[from] TransformError),
}

struct ElementFrame<'src, 'a> {
    name: &'src str,
    span: SvgSpan,
    opening_span: SvgSpan,
    name_span: SvgSpan,
    attributes: ArenaVec<'a, JSXAttributeItem<'a>>,
    children: ArenaVec<'a, JSXChild<'a>>,
}

impl<'src, 'a> SvgSink<'src> for OxcJsxSink<'src, 'a> {
    type Error = SinkError;

    fn start_element(&mut self, event: StartElement<'src>) -> Result<(), Self::Error> {
        self.stack.push(ElementFrame {
            name: event.name,
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
        let item = self.attr_item(attr, current.name);
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
            let is_root = self.stack.is_empty() && self.root.is_none();
            if let Some(child) = self.build_element(frame, None, is_root)? {
                self.attach_child(child)?;
            }
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
                expected: frame.name.into(),
                found: event.name.into(),
            });
        }
        frame.span.end = event.span.end;
        let is_root = self.stack.is_empty() && self.root.is_none();
        if let Some(child) = self.build_element(frame, Some(event.span), is_root)? {
            self.attach_child(child)?;
        }
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

fn build_component_program<'a>(
    allocator: &'a Allocator,
    jsx: Expression<'a>,
    options: &TransformOptions,
) -> Result<Program<'a>, TransformError> {
    let ast = AstBuilder::new(allocator);
    let native_components = if options.native {
        collect_native_components(&jsx)
    } else {
        BTreeSet::new()
    };

    let mut body = ast.vec();
    append_imports(allocator, &mut body, options, &native_components)?;
    if options.typescript && (options.title_prop || options.desc_prop) {
        body.push(svgr_props_interface_statement(allocator, options));
    }
    body.push(component_statement(
        allocator,
        options.component_name.as_str(),
        jsx,
        options,
    ));

    let mut export_identifier = options.component_name.as_str();
    if options.r#ref {
        // Wrapper binding collision handling was considered, but is intentionally not implemented
        // yet; keep the legacy ForwardRef/Memo names for now.
        body.push(call_wrapper_statement(
            allocator,
            "ForwardRef",
            "forwardRef",
            export_identifier,
        ));
        export_identifier = "ForwardRef";
    }
    if options.memo {
        body.push(call_wrapper_statement(
            allocator,
            "Memo",
            "memo",
            export_identifier,
        ));
        export_identifier = "Memo";
    }

    if options.previous_export.is_some() || options.export_type == ExportType::Named {
        body.push(export_named_statement(
            allocator,
            export_identifier,
            options.named_export.as_str(),
        ));
        if let Some(previous_export) = &options.previous_export {
            let previous_body = parse_previous_export(allocator, previous_export, options)?;
            for statement in previous_body {
                body.push(statement);
            }
        }
    } else {
        body.push(export_default_statement(allocator, export_identifier));
    }

    Ok(ast.program(
        SPAN,
        source_type(options.typescript),
        "",
        ast.vec(),
        None,
        ast.vec(),
        body,
    ))
}

fn append_imports<'a>(
    allocator: &'a Allocator,
    body: &mut ArenaVec<'a, Statement<'a>>,
    options: &TransformOptions,
    native_components: &BTreeSet<String>,
) -> Result<(), TransformError> {
    let ast = AstBuilder::new(allocator);
    if options.jsx_runtime != JsxRuntime::Automatic {
        let import = options
            .jsx_runtime_import
            .clone()
            .unwrap_or_else(default_jsx_runtime_import);
        let mut specifiers = ast.vec();
        if let Some(namespace) = import.namespace {
            specifiers.push(import_namespace_specifier(allocator, namespace.as_str()));
        } else if let Some(default_specifier) = import.default_specifier {
            specifiers.push(import_default_specifier(
                allocator,
                default_specifier.as_str(),
            ));
        } else {
            for specifier in &import.specifiers {
                specifiers.push(import_named_specifier(
                    allocator,
                    specifier.as_str(),
                    ImportOrExportKind::Value,
                ));
            }
        }
        if specifiers.is_empty() {
            return Err(TransformError::InvalidOptions(
                "jsxRuntimeImport requires namespace, defaultSpecifier, or specifiers".into(),
            ));
        }
        body.push(import_statement(
            allocator,
            import.source.as_str(),
            specifiers,
            ImportOrExportKind::Value,
        ));
    }

    if options.native {
        let mut specifiers = ast.vec();
        specifiers.push(import_default_specifier(allocator, "Svg"));
        for component in native_components {
            specifiers.push(import_named_specifier(
                allocator,
                component.as_str(),
                ImportOrExportKind::Value,
            ));
        }
        body.push(import_statement(
            allocator,
            "react-native-svg",
            specifiers,
            ImportOrExportKind::Value,
        ));
    }

    let mut react_value_imports = ast.vec();
    if options.r#ref {
        react_value_imports.push(import_named_specifier(
            allocator,
            "forwardRef",
            ImportOrExportKind::Value,
        ));
    }
    if options.memo {
        react_value_imports.push(import_named_specifier(
            allocator,
            "memo",
            ImportOrExportKind::Value,
        ));
    }
    if !react_value_imports.is_empty() {
        body.push(import_statement(
            allocator,
            options.import_source.as_str(),
            react_value_imports,
            ImportOrExportKind::Value,
        ));
    }

    if options.typescript && (options.expand_props != ExpandProps::Disabled || options.r#ref) {
        let mut type_imports = ast.vec();
        if options.expand_props != ExpandProps::Disabled {
            type_imports.push(import_named_specifier(
                allocator,
                if options.native {
                    "SvgProps"
                } else {
                    "SVGProps"
                },
                ImportOrExportKind::Value,
            ));
        }
        if options.r#ref && !options.native {
            type_imports.push(import_named_specifier(
                allocator,
                "Ref",
                ImportOrExportKind::Value,
            ));
        }
        if !type_imports.is_empty() {
            body.push(import_statement(
                allocator,
                if options.native {
                    "react-native-svg"
                } else {
                    options.import_source.as_str()
                },
                type_imports,
                ImportOrExportKind::Type,
            ));
        }
    }
    Ok(())
}

fn import_statement<'a>(
    allocator: &'a Allocator,
    source: &str,
    specifiers: ArenaVec<'a, ImportDeclarationSpecifier<'a>>,
    import_kind: ImportOrExportKind,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    Statement::ImportDeclaration(ast.alloc_import_declaration(
        SPAN,
        Some(specifiers),
        ast.string_literal(SPAN, ast.str(source), None),
        None,
        NONE,
        import_kind,
    ))
}

fn import_named_specifier<'a>(
    allocator: &'a Allocator,
    name: &str,
    import_kind: ImportOrExportKind,
) -> ImportDeclarationSpecifier<'a> {
    let ast = AstBuilder::new(allocator);
    ast.import_declaration_specifier_import_specifier(
        SPAN,
        ast.module_export_name_identifier_name(SPAN, ast.str(name)),
        ast.binding_identifier(SPAN, ast.str(name)),
        import_kind,
    )
}

fn import_default_specifier<'a>(
    allocator: &'a Allocator,
    local: &str,
) -> ImportDeclarationSpecifier<'a> {
    let ast = AstBuilder::new(allocator);
    ast.import_declaration_specifier_import_default_specifier(
        SPAN,
        ast.binding_identifier(SPAN, ast.str(local)),
    )
}

fn import_namespace_specifier<'a>(
    allocator: &'a Allocator,
    local: &str,
) -> ImportDeclarationSpecifier<'a> {
    let ast = AstBuilder::new(allocator);
    ast.import_declaration_specifier_import_namespace_specifier(
        SPAN,
        ast.binding_identifier(SPAN, ast.str(local)),
    )
}

fn component_statement<'a>(
    allocator: &'a Allocator,
    component_name: &str,
    jsx: Expression<'a>,
    options: &TransformOptions,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    let mut statements = ast.vec();
    statements.push(ast.statement_expression(SPAN, jsx));
    let arrow = ast.expression_arrow_function(
        SPAN,
        true,
        false,
        NONE,
        component_params(allocator, options),
        NONE,
        ast.function_body(SPAN, ast.vec(), statements),
    );
    variable_statement(allocator, component_name, arrow)
}

fn call_wrapper_statement<'a>(
    allocator: &'a Allocator,
    binding_name: &str,
    callee_name: &str,
    argument_name: &str,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    let mut arguments = ast.vec();
    arguments.push(Argument::from(
        ast.expression_identifier(SPAN, ast.str(argument_name)),
    ));
    let call = ast.expression_call(
        SPAN,
        ast.expression_identifier(SPAN, ast.str(callee_name)),
        NONE,
        arguments,
        false,
    );
    variable_statement(allocator, binding_name, call)
}

fn variable_statement<'a>(
    allocator: &'a Allocator,
    binding_name: &str,
    init: Expression<'a>,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    let mut declarations = ast.vec();
    declarations.push(ast.variable_declarator(
        SPAN,
        VariableDeclarationKind::Const,
        ast.binding_pattern_binding_identifier(SPAN, ast.str(binding_name)),
        NONE,
        Some(init),
        false,
    ));
    Statement::VariableDeclaration(ast.alloc_variable_declaration(
        SPAN,
        VariableDeclarationKind::Const,
        declarations,
        false,
    ))
}

fn export_named_statement<'a>(
    allocator: &'a Allocator,
    local: &str,
    exported: &str,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    let mut specifiers = ast.vec();
    specifiers.push(ast.export_specifier(
        SPAN,
        ast.module_export_name_identifier_reference(SPAN, ast.str(local)),
        ast.module_export_name_identifier_name(SPAN, ast.str(exported)),
        ImportOrExportKind::Value,
    ));
    Statement::ExportNamedDeclaration(ast.alloc_export_named_declaration(
        SPAN,
        None,
        specifiers,
        None,
        ImportOrExportKind::Value,
        NONE,
    ))
}

fn export_default_statement<'a>(allocator: &'a Allocator, local: &str) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    Statement::ExportDefaultDeclaration(ast.alloc_export_default_declaration(
        SPAN,
        ExportDefaultDeclarationKind::from(ast.expression_identifier(SPAN, ast.str(local))),
    ))
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

fn parse_previous_export<'a>(
    allocator: &'a Allocator,
    source: &str,
    options: &TransformOptions,
) -> Result<ArenaVec<'a, Statement<'a>>, TransformError> {
    let program = parse_program(allocator, source.trim(), options.typescript).map_err(|error| {
        TransformError::InvalidOptions(format!("invalid previousExport: {error}"))
    })?;
    if body_exports_name(&program.body, options.named_export.as_str()) {
        return Err(TransformError::InvalidOptions(format!(
            "previousExport already exports `{}`",
            options.named_export
        )));
    }
    Ok(program.body)
}

fn body_exports_name(body: &ArenaVec<'_, Statement<'_>>, name: &str) -> bool {
    body.iter()
        .any(|statement| statement_exports_name(statement, name))
}

fn statement_exports_name(statement: &Statement<'_>, name: &str) -> bool {
    let Statement::ExportNamedDeclaration(declaration) = statement else {
        return false;
    };
    declaration
        .specifiers
        .iter()
        .any(|specifier| module_export_name_matches(&specifier.exported, name))
        || declaration
            .declaration
            .as_ref()
            .is_some_and(|declaration| declaration_exports_name(declaration, name))
}

fn declaration_exports_name(declaration: &Declaration<'_>, name: &str) -> bool {
    match declaration {
        Declaration::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .any(|declarator| binding_pattern_contains_name(&declarator.id, name)),
        Declaration::FunctionDeclaration(function) => function
            .id
            .as_ref()
            .is_some_and(|id| id.name.as_str() == name),
        Declaration::ClassDeclaration(class) => {
            class.id.as_ref().is_some_and(|id| id.name.as_str() == name)
        }
        Declaration::TSInterfaceDeclaration(interface) => interface.id.name.as_str() == name,
        _ => false,
    }
}

fn binding_pattern_contains_name(pattern: &BindingPattern<'_>, name: &str) -> bool {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => identifier.name.as_str() == name,
        BindingPattern::ObjectPattern(pattern) => {
            pattern
                .properties
                .iter()
                .any(|property| binding_pattern_contains_name(&property.value, name))
                || pattern
                    .rest
                    .as_ref()
                    .is_some_and(|rest| binding_pattern_contains_name(&rest.argument, name))
        }
        BindingPattern::ArrayPattern(pattern) => {
            pattern.elements.iter().any(|element| {
                element
                    .as_ref()
                    .is_some_and(|element| binding_pattern_contains_name(element, name))
            }) || pattern
                .rest
                .as_ref()
                .is_some_and(|rest| binding_pattern_contains_name(&rest.argument, name))
        }
        BindingPattern::AssignmentPattern(pattern) => {
            binding_pattern_contains_name(&pattern.left, name)
        }
    }
}

fn module_export_name_matches(export_name: &ModuleExportName<'_>, name: &str) -> bool {
    match export_name {
        ModuleExportName::IdentifierName(identifier) => identifier.name.as_str() == name,
        ModuleExportName::IdentifierReference(identifier) => identifier.name.as_str() == name,
        ModuleExportName::StringLiteral(literal) => literal.value.as_str() == name,
    }
}

fn validate_options(options: &TransformOptions) -> Result<(), TransformError> {
    validate_binding_name("componentName", options.component_name.as_str())?;
    validate_binding_name("namedExport", options.named_export.as_str())?;
    if let Some(import) = &options.jsx_runtime_import {
        if import.namespace.is_none()
            && import.default_specifier.is_none()
            && import.specifiers.is_empty()
        {
            return Err(TransformError::InvalidOptions(
                "jsxRuntimeImport requires namespace, defaultSpecifier, or specifiers".into(),
            ));
        }
        if let Some(namespace) = &import.namespace {
            validate_binding_name("jsxRuntimeImport.namespace", namespace)?;
        }
        if let Some(default_specifier) = &import.default_specifier {
            validate_binding_name("jsxRuntimeImport.defaultSpecifier", default_specifier)?;
        }
        for specifier in &import.specifiers {
            validate_binding_name("jsxRuntimeImport.specifiers[]", specifier)?;
        }
    }
    Ok(())
}

fn validate_binding_name(option: &str, value: &str) -> Result<(), TransformError> {
    if !is_identifier_name(value) || is_reserved_keyword(value) {
        return Err(TransformError::InvalidOptions(format!(
            "{option} must be a valid JavaScript binding identifier"
        )));
    }
    Ok(())
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

fn component_params<'a>(
    allocator: &'a Allocator,
    options: &TransformOptions,
) -> oxc_allocator::Box<'a, FormalParameters<'a>> {
    let ast = AstBuilder::new(allocator);
    let mut items = ast.vec();
    let mut props = Vec::new();
    if options.title_prop {
        props.push("title");
        props.push("titleId");
    }
    if options.desc_prop {
        props.push("desc");
        props.push("descId");
    }

    if props.is_empty() {
        if options.expand_props == ExpandProps::Disabled {
            if options.r#ref {
                items.push(formal_parameter(
                    allocator,
                    ast.binding_pattern_binding_identifier(SPAN, ast.str("_")),
                    None,
                ));
            }
        } else {
            items.push(formal_parameter(
                allocator,
                ast.binding_pattern_binding_identifier(SPAN, ast.str("props")),
                props_type_annotation(allocator, options),
            ));
        }
    } else {
        let mut properties = ast.vec();
        for prop in props {
            properties.push(ast.binding_property(
                SPAN,
                ast.property_key_static_identifier(SPAN, ast.str(prop)),
                ast.binding_pattern_binding_identifier(SPAN, ast.str(prop)),
                true,
                false,
            ));
        }
        let rest = if options.expand_props != ExpandProps::Disabled {
            Some(ast.alloc_binding_rest_element(
                SPAN,
                ast.binding_pattern_binding_identifier(SPAN, ast.str("props")),
            ))
        } else {
            None
        };
        items.push(formal_parameter(
            allocator,
            ast.binding_pattern_object_pattern(SPAN, properties, rest),
            props_type_annotation(allocator, options),
        ));
    }

    if options.r#ref {
        items.push(formal_parameter(
            allocator,
            ast.binding_pattern_binding_identifier(SPAN, ast.str("ref")),
            ref_type_annotation(allocator, options),
        ));
    }

    ast.alloc_formal_parameters(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        items,
        None::<oxc_allocator::Box<'a, FormalParameterRest<'a>>>,
    )
}

fn formal_parameter<'a>(
    allocator: &'a Allocator,
    pattern: BindingPattern<'a>,
    type_annotation: Option<oxc_allocator::Box<'a, TSTypeAnnotation<'a>>>,
) -> FormalParameter<'a> {
    let ast = AstBuilder::new(allocator);
    ast.formal_parameter(
        SPAN,
        ast.vec(),
        pattern,
        type_annotation,
        None::<oxc_allocator::Box<'a, Expression<'a>>>,
        false,
        None,
        false,
        false,
    )
}

fn props_type_annotation<'a>(
    allocator: &'a Allocator,
    options: &TransformOptions,
) -> Option<oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
    if !options.typescript {
        return None;
    }
    let ast = AstBuilder::new(allocator);
    let mut types = ast.vec();
    if options.expand_props != ExpandProps::Disabled {
        types.push(svg_props_type(allocator, options.native));
    }
    if options.title_prop || options.desc_prop {
        types.push(ts_reference_type(allocator, "SVGRProps", None));
    }
    match types.len() {
        0 => None,
        1 => types
            .into_iter()
            .next()
            .map(|ty| ast.alloc_ts_type_annotation(SPAN, ty)),
        _ => Some(ast.alloc_ts_type_annotation(SPAN, ast.ts_type_intersection_type(SPAN, types))),
    }
}

fn ref_type_annotation<'a>(
    allocator: &'a Allocator,
    options: &TransformOptions,
) -> Option<oxc_allocator::Box<'a, TSTypeAnnotation<'a>>> {
    if !options.typescript || options.native {
        return None;
    }
    let ast = AstBuilder::new(allocator);
    let mut type_arguments = ast.vec();
    type_arguments.push(ts_reference_type(allocator, "SVGSVGElement", None));
    Some(ast.alloc_ts_type_annotation(
        SPAN,
        ts_reference_type(allocator, "Ref", Some(type_arguments)),
    ))
}

fn svg_props_type<'a>(allocator: &'a Allocator, native: bool) -> TSType<'a> {
    if native {
        ts_reference_type(allocator, "SvgProps", None)
    } else {
        let ast = AstBuilder::new(allocator);
        let mut type_arguments = ast.vec();
        type_arguments.push(ts_reference_type(allocator, "SVGSVGElement", None));
        ts_reference_type(allocator, "SVGProps", Some(type_arguments))
    }
}

fn ts_reference_type<'a>(
    allocator: &'a Allocator,
    name: &str,
    type_arguments: Option<ArenaVec<'a, TSType<'a>>>,
) -> TSType<'a> {
    let ast = AstBuilder::new(allocator);
    let type_arguments =
        type_arguments.map(|params| ast.alloc_ts_type_parameter_instantiation(SPAN, params));
    ast.ts_type_type_reference(
        SPAN,
        ast.ts_type_name_identifier_reference(SPAN, ast.str(name)),
        type_arguments,
    )
}

fn svgr_props_interface_statement<'a>(
    allocator: &'a Allocator,
    options: &TransformOptions,
) -> Statement<'a> {
    let ast = AstBuilder::new(allocator);
    let mut body = ast.vec();
    if options.title_prop {
        body.push(ts_string_property_signature(allocator, "title"));
        body.push(ts_string_property_signature(allocator, "titleId"));
    }
    if options.desc_prop {
        body.push(ts_string_property_signature(allocator, "desc"));
        body.push(ts_string_property_signature(allocator, "descId"));
    }
    Statement::from(ast.declaration_ts_interface(
        SPAN,
        ast.binding_identifier(SPAN, ast.str("SVGRProps")),
        NONE,
        ast.vec(),
        ast.ts_interface_body(SPAN, body),
        false,
    ))
}

fn ts_string_property_signature<'a>(allocator: &'a Allocator, name: &str) -> TSSignature<'a> {
    let ast = AstBuilder::new(allocator);
    ast.ts_signature_property_signature(
        SPAN,
        false,
        true,
        false,
        ast.property_key_static_identifier(SPAN, ast.str(name)),
        Some(ast.alloc_ts_type_annotation(SPAN, ast.ts_type_string_keyword(SPAN))),
    )
}

fn default_jsx_runtime_import() -> JsxRuntimeImport {
    JsxRuntimeImport {
        source: "react".into(),
        namespace: Some("React".into()),
        default_specifier: None,
        specifiers: Vec::new(),
    }
}

fn map_element_name(name: &str) -> Cow<'_, str> {
    match name {
        "clippath" => Cow::Borrowed("clipPath"),
        "lineargradient" => Cow::Borrowed("linearGradient"),
        "radialgradient" => Cow::Borrowed("radialGradient"),
        "textpath" => Cow::Borrowed("textPath"),
        "foreignobject" => Cow::Borrowed("foreignObject"),
        "feblend" => Cow::Borrowed("feBlend"),
        "fecolormatrix" => Cow::Borrowed("feColorMatrix"),
        "fecomponenttransfer" => Cow::Borrowed("feComponentTransfer"),
        "fecomposite" => Cow::Borrowed("feComposite"),
        "feconvolvematrix" => Cow::Borrowed("feConvolveMatrix"),
        "fediffuselighting" => Cow::Borrowed("feDiffuseLighting"),
        "fedisplacementmap" => Cow::Borrowed("feDisplacementMap"),
        "fedistantlight" => Cow::Borrowed("feDistantLight"),
        "fedropshadow" => Cow::Borrowed("feDropShadow"),
        "feflood" => Cow::Borrowed("feFlood"),
        "fefunca" => Cow::Borrowed("feFuncA"),
        "fefuncb" => Cow::Borrowed("feFuncB"),
        "fefuncg" => Cow::Borrowed("feFuncG"),
        "fefuncr" => Cow::Borrowed("feFuncR"),
        "fegaussianblur" => Cow::Borrowed("feGaussianBlur"),
        "feimage" => Cow::Borrowed("feImage"),
        "femerge" => Cow::Borrowed("feMerge"),
        "femergenode" => Cow::Borrowed("feMergeNode"),
        "femorphology" => Cow::Borrowed("feMorphology"),
        "feoffset" => Cow::Borrowed("feOffset"),
        "fepointlight" => Cow::Borrowed("fePointLight"),
        "fespecularlighting" => Cow::Borrowed("feSpecularLighting"),
        "fespotlight" => Cow::Borrowed("feSpotLight"),
        "fetile" => Cow::Borrowed("feTile"),
        "feturbulence" => Cow::Borrowed("feTurbulence"),
        _ => Cow::Borrowed(name),
    }
}

fn map_attribute_name<'n>(name: &'n str, element_name: &str) -> Cow<'n, str> {
    if element_name == "input" && name.eq_ignore_ascii_case("checked") {
        return Cow::Borrowed("defaultChecked");
    }
    if element_name == "input" && name.eq_ignore_ascii_case("value") {
        return Cow::Borrowed("defaultValue");
    }
    if name.eq_ignore_ascii_case("class") || name.eq_ignore_ascii_case("classname") {
        return Cow::Borrowed("className");
    }
    if name.eq_ignore_ascii_case("for") {
        return Cow::Borrowed("htmlFor");
    }
    if name.eq_ignore_ascii_case("tabindex") {
        return Cow::Borrowed("tabIndex");
    }
    if name.eq_ignore_ascii_case("viewbox") {
        return Cow::Borrowed("viewBox");
    }
    if name.eq_ignore_ascii_case("preserveaspectratio") {
        return Cow::Borrowed("preserveAspectRatio");
    }
    if name.eq_ignore_ascii_case("xmlns:xlink") || name.eq_ignore_ascii_case("xmlnsxlink") {
        return Cow::Borrowed("xmlnsXlink");
    }
    if name.eq_ignore_ascii_case("xml:space") || name.eq_ignore_ascii_case("xmlspace") {
        return Cow::Borrowed("xmlSpace");
    }
    if name.eq_ignore_ascii_case("xml:lang") || name.eq_ignore_ascii_case("xmllang") {
        return Cow::Borrowed("xmlLang");
    }
    if name.eq_ignore_ascii_case("xlink:href") || name.eq_ignore_ascii_case("xlinkhref") {
        return Cow::Borrowed("xlinkHref");
    }
    if name.eq_ignore_ascii_case("xlink:title") || name.eq_ignore_ascii_case("xlinktitle") {
        return Cow::Borrowed("xlinkTitle");
    }
    if name.eq_ignore_ascii_case("xlink:type") || name.eq_ignore_ascii_case("xlinktype") {
        return Cow::Borrowed("xlinkType");
    }
    if starts_with_ignore_ascii_case(name, "aria-") {
        return convert_aria_attribute(name);
    }
    if starts_with_ignore_ascii_case(name, "data-") {
        return if name.bytes().all(|byte| !byte.is_ascii_uppercase()) {
            Cow::Borrowed(name)
        } else {
            Cow::Owned(name.to_ascii_lowercase())
        };
    }
    camelize_svg_name(name)
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn camelize_svg_name(name: &str) -> Cow<'_, str> {
    if !name
        .as_bytes()
        .iter()
        .any(|&byte| byte == b'-' || byte == b':')
    {
        return Cow::Borrowed(name);
    }
    let mut result = String::with_capacity(name.len());
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
    Cow::Owned(result)
}

fn convert_aria_attribute(name: &str) -> Cow<'_, str> {
    let mut parts = name.split('-');
    let first = parts.next().unwrap_or_default();
    let rest = parts.collect::<String>().to_ascii_lowercase();
    Cow::Owned(format!("{}-{rest}", first.to_ascii_lowercase()))
}

fn style_to_object_expression<'a>(allocator: &'a Allocator, raw: &str) -> Option<Expression<'a>> {
    let ast = AstBuilder::new(allocator);
    let mut props = ast.vec();
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
            Cow::Borrowed(key)
        } else {
            camelize_svg_name(key.trim_start_matches("-ms-"))
        };
        let value = value.trim();
        let value = if let Some(px) = value.strip_suffix("px").and_then(|v| numeric_value(v)) {
            ast.expression_numeric_literal(SPAN, px, None, NumberBase::Decimal)
        } else if let Some(number) = numeric_value(value) {
            ast.expression_numeric_literal(SPAN, number, None, NumberBase::Decimal)
        } else {
            ast.expression_string_literal(SPAN, ast.str(value), None)
        };
        let key = if key.starts_with("--") || !is_identifier_name(formatted_key.as_ref()) {
            PropertyKey::StringLiteral(ast.alloc_string_literal(
                SPAN,
                ast.str(formatted_key.as_ref()),
                None,
            ))
        } else {
            ast.property_key_static_identifier(SPAN, ast.str(formatted_key.as_ref()))
        };
        props.push(ObjectPropertyKind::ObjectProperty(
            ast.alloc_object_property(SPAN, PropertyKind::Init, key, value, false, false, false),
        ));
    }
    Some(ast.expression_object(SPAN, props))
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

fn is_always_string_attribute(name: &str) -> bool {
    matches!(
        name,
        "d" | "points"
            | "viewBox"
            | "preserveAspectRatio"
            | "transform"
            | "gradientTransform"
            | "patternTransform"
    )
}

fn normalize_attribute_value(value: &str) -> Cow<'_, str> {
    if value
        .as_bytes()
        .iter()
        .all(|byte| byte.is_ascii() && !matches!(byte, b'&' | b'\t' | b'\r' | b'\n'))
    {
        return Cow::Borrowed(value);
    }
    let decoded = decode_xml(value);
    if !decoded.chars().any(is_xml_space_replacement) {
        return decoded;
    }
    decoded
        .chars()
        .map(|ch| {
            if is_xml_space_replacement(ch) {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .into()
}

fn is_xml_space_replacement(ch: char) -> bool {
    matches!(
        ch,
        '\t' | '\r' | '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

fn decode_xml(value: &str) -> Cow<'_, str> {
    let Some(first_entity) = value.find('&') else {
        return Cow::Borrowed(value);
    };
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    let mut next_entity = Some(first_entity);
    while let Some(index) = next_entity {
        result.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        let Some(end) = rest.find(';') else {
            result.push('&');
            result.push_str(rest);
            return Cow::Owned(result);
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
        next_entity = rest.find('&');
    }
    result.push_str(rest);
    Cow::Owned(result)
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

    #[test]
    fn dimensions_false_removes_source_dimensions_before_adding_svg_props() {
        let options = TransformOptions {
            dimensions: false,
            svg_props: vec![("width".into(), "1em".into())],
            ..TransformOptions::default()
        };
        let result = code(
            r#"<svg width="10" height="20" viewBox="0 0 10 20" />"#,
            options,
        );

        assert!(result.contains("<svg viewBox=\"0 0 10 20\" width=\"1em\" {...props} />"));
        assert!(!result.contains("height="));
    }

    #[test]
    fn replace_attr_values_applies_to_source_and_added_svg_attributes() {
        let options = TransformOptions {
            svg_props: vec![("role".into(), "img".into())],
            replace_attr_values: vec![
                ("#fff".into(), "{props.color}".into()),
                ("img".into(), "presentation".into()),
            ],
            ..TransformOptions::default()
        };
        let result = code(r##"<svg fill="#fff" />"##, options);

        assert!(result.contains("fill={props.color}"));
        assert!(result.contains("role=\"presentation\""));
    }

    #[test]
    fn preserves_mapping_and_normalization_after_lazy_string_paths() {
        let result = code(
            r#"<svg viewbox="0 0 1 1" DATA-ID="Logo" aria-LABEL="Icon" title="A&#10;B"><title>Tom &amp; Jerry</title><path stroke-width="2" style="stroke-width: 2px; --brand-color: red" /></svg>"#,
            TransformOptions::default(),
        );

        assert!(result.contains("viewBox=\"0 0 1 1\""));
        assert!(result.contains("data-id=\"Logo\""));
        assert!(result.contains("aria-label=\"Icon\""));
        assert!(result.contains("title=\"A B\""));
        assert!(result.contains("<title>{\"Tom & Jerry\"}</title>"));
        assert!(result.contains("strokeWidth={2}"));
        assert!(result.contains("strokeWidth: 2"));
        assert!(result.contains("\"--brand-color\": \"red\""));
    }

    #[test]
    fn keeps_structured_svg_attributes_as_strings() {
        let result = code(
            r#"<svg viewBox="1"><path d="1" transform="1" /><polyline points="1" /></svg>"#,
            TransformOptions::default(),
        );

        assert!(result.contains("viewBox=\"1\""));
        assert!(result.contains("d=\"1\""));
        assert!(result.contains("transform=\"1\""));
        assert!(result.contains("points=\"1\""));
    }

    #[test]
    fn borrows_attribute_values_that_need_no_normalization() {
        assert!(matches!(
            normalize_attribute_value("M0 0 L10 10"),
            Cow::Borrowed(_)
        ));
        assert_eq!(normalize_attribute_value("A&#10;B"), "A B");
        assert_eq!(normalize_attribute_value("A\u{2028}B"), "A B");
    }

    #[test]
    fn native_drops_unsupported_subtrees_during_sink_time() {
        let options = TransformOptions {
            native: true,
            ..TransformOptions::default()
        };
        let result = code(
            r#"<svg><g><div><path d="M0 0" /></div><path d="M1 1" /></g></svg>"#,
            options,
        );

        assert!(result.contains("import Svg, { G, Path } from \"react-native-svg\";"));
        assert!(result.contains("<Svg {...props}><G><Path d=\"M1 1\" /></G></Svg>"));
        assert!(!result.contains("M0 0"));
        assert!(!result.contains("div"));
    }

    #[test]
    fn native_preserves_existing_uppercase_svg_root() {
        let options = TransformOptions {
            native: true,
            ..TransformOptions::default()
        };
        let result = code(r#"<Svg><path d="M0 0" /></Svg>"#, options);

        assert!(result.contains("import Svg, { Path } from \"react-native-svg\";"));
        assert!(result.contains("<Svg {...props}><Path d=\"M0 0\" /></Svg>"));
    }

    #[test]
    fn native_keeps_dynamic_title_fallback_when_title_prop_is_enabled() {
        let options = TransformOptions {
            native: true,
            title_prop: true,
            ..TransformOptions::default()
        };
        let result = code(
            r#"<svg><title id="a">Hello</title><path d="M0 0" /></svg>"#,
            options,
        );

        assert!(result.contains("import Svg, { Path } from \"react-native-svg\";"));
        assert!(result.contains("title === undefined ? <title id={titleId || \"a\"}>"));
        assert!(result.contains("<Path d=\"M0 0\" />"));
    }

    #[test]
    fn rejects_invalid_identifier_options() {
        let err = transform(
            r#"<svg />"#,
            TransformOptions {
                component_name: "Svg;Component".into(),
                ..TransformOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, TransformError::InvalidOptions(_)));

        let err = transform(
            r#"<svg />"#,
            TransformOptions {
                jsx_runtime_import: Some(JsxRuntimeImport {
                    source: "preact".into(),
                    namespace: None,
                    default_specifier: None,
                    specifiers: vec!["hacked;".into()],
                }),
                ..TransformOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, TransformError::InvalidOptions(_)));
    }

    #[test]
    fn parses_previous_export_as_statements() {
        let result = code(
            r#"<svg />"#,
            TransformOptions {
                named_export: "Component".into(),
                previous_export: Some(r#"const img = "logo.svg"; export default img;"#.into()),
                ..TransformOptions::default()
            },
        );
        assert!(result.contains("export { SvgComponent as Component };"));
        assert!(result.contains("const img = \"logo.svg\";"));
        assert!(result.contains("export default img;"));
    }

    #[test]
    fn rejects_invalid_or_conflicting_previous_export() {
        let err = transform(
            r#"<svg />"#,
            TransformOptions {
                previous_export: Some("export default ;".into()),
                ..TransformOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, TransformError::InvalidOptions(_)));

        let err = transform(
            r#"<svg />"#,
            TransformOptions {
                export_type: ExportType::Named,
                named_export: "Component".into(),
                previous_export: Some("export { value as Component };".into()),
                ..TransformOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, TransformError::InvalidOptions(_)));
    }
}
