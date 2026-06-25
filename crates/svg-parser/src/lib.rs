use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}

pub fn line_col(source: &str, offset: usize) -> LineCol {
    let capped = offset.min(source.len());
    let mut line = 1;
    let mut column = 1;

    for byte in source[..capped].bytes() {
        if byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    LineCol { line, column }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    Double,
    Single,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartElement<'src> {
    pub name: &'src str,
    pub span: Span,
    pub name_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute<'src> {
    pub name: &'src str,
    pub value: Option<&'src str>,
    pub span: Span,
    pub name_span: Span,
    pub value_span: Option<Span>,
    pub quote: Option<QuoteKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishStartElement {
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndElement<'src> {
    pub name: &'src str,
    pub span: Span,
    pub name_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comment<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CData<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doctype<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessingInstruction<'src> {
    pub value: &'src str,
    pub span: Span,
}

pub trait SvgSink<'src> {
    type Error;

    fn start_element(&mut self, event: StartElement<'src>) -> Result<(), Self::Error>;

    fn attribute(&mut self, attr: Attribute<'src>) -> Result<(), Self::Error>;

    fn finish_start_element(&mut self, event: FinishStartElement) -> Result<(), Self::Error>;

    fn end_element(&mut self, event: EndElement<'src>) -> Result<(), Self::Error>;

    fn text(&mut self, text: Text<'src>) -> Result<(), Self::Error>;

    fn comment(&mut self, comment: Comment<'src>) -> Result<(), Self::Error> {
        let _ = comment;
        Ok(())
    }

    fn cdata(&mut self, cdata: CData<'src>) -> Result<(), Self::Error> {
        let _ = cdata;
        Ok(())
    }

    fn doctype(&mut self, doctype: Doctype<'src>) -> Result<(), Self::Error> {
        let _ = doctype;
        Ok(())
    }

    fn processing_instruction(
        &mut self,
        instruction: ProcessingInstruction<'src>,
    ) -> Result<(), Self::Error> {
        let _ = instruction;
        Ok(())
    }

    fn finish_document(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError<E> {
    UnexpectedEof { span: Span },
    UnexpectedCharacter { byte: Option<u8>, span: Span },
    ExpectedTagName { span: Span },
    ExpectedAttributeName { span: Span },
    ExpectedGt { span: Span },
    UnclosedQuote { quote: QuoteKind, span: Span },
    ExpectedCommentEnd { span: Span },
    ExpectedCDataEnd { span: Span },
    ExpectedProcessingInstructionEnd { span: Span },
    Sink(E),
}

impl<E: fmt::Display> fmt::Display for ParseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { .. } => {
                write!(f, "unexpected end of input")
            }
            Self::UnexpectedCharacter { byte, .. } => match byte {
                Some(byte) => write!(f, "unexpected character `{}`", *byte as char),
                None => write!(f, "unexpected end of input"),
            },
            Self::ExpectedTagName { .. } => {
                write!(f, "expected tag name")
            }
            Self::ExpectedAttributeName { .. } => {
                write!(f, "expected attribute name")
            }
            Self::ExpectedGt { .. } => {
                write!(f, "expected `>`")
            }
            Self::UnclosedQuote { quote, .. } => {
                write!(f, "unclosed {:?} quote", quote)
            }
            Self::ExpectedCommentEnd { .. } => {
                write!(f, "expected comment end `-->`")
            }
            Self::ExpectedCDataEnd { .. } => {
                write!(f, "expected CDATA end `]]>`")
            }
            Self::ExpectedProcessingInstructionEnd { .. } => {
                write!(f, "expected processing instruction end `?>`")
            }
            Self::Sink(error) => {
                write!(f, "sink error: {error}")
            }
        }
    }
}

impl<E> ParseError<E> {
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::UnexpectedEof { span }
            | Self::UnexpectedCharacter { span, .. }
            | Self::ExpectedTagName { span }
            | Self::ExpectedAttributeName { span }
            | Self::ExpectedGt { span }
            | Self::UnclosedQuote { span, .. }
            | Self::ExpectedCommentEnd { span }
            | Self::ExpectedCDataEnd { span }
            | Self::ExpectedProcessingInstructionEnd { span } => Some(*span),
            Self::Sink(_) => None,
        }
    }

    pub fn diagnostic(&self, source: &str) -> Diagnostic
    where
        E: fmt::Display,
    {
        let span = self.span().unwrap_or_default();
        let LineCol { line, column } = line_col(source, span.start);

        Diagnostic {
            message: self.to_string(),
            line,
            column,
            span,
        }
    }
}

pub fn parse_with_sink<'src, S>(source: &'src str, sink: &mut S) -> Result<(), ParseError<S::Error>>
where
    S: SvgSink<'src>,
{
    let mut parser = Parser::new(source, sink);
    parser.parse_document()
}

pub fn parse<'src>(
    source: &'src str,
) -> Result<SvgDocument<'src>, ParseError<TreeSinkError<'src>>> {
    let mut sink = TreeSink::default();
    parse_with_sink(source, &mut sink)?;
    Ok(sink.finish())
}

struct Parser<'src, 'sink, S> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
    sink: &'sink mut S,
}

impl<'src, 'sink, S> Parser<'src, 'sink, S>
where
    S: SvgSink<'src>,
{
    fn new(source: &'src str, sink: &'sink mut S) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            sink,
        }
    }

    fn parse_document(&mut self) -> Result<(), ParseError<S::Error>> {
        while !self.eof() {
            if self.current() == Some(b'<') {
                self.pos += 1;
                self.parse_tag()?;
            } else {
                self.parse_text()?;
            }
        }

        self.sink.finish_document().map_err(ParseError::Sink)?;

        Ok(())
    }

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn current(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn starts_with(&self, pattern: &[u8]) -> bool {
        self.bytes[self.pos..].starts_with(pattern)
    }

    fn span_at_current(&self) -> Span {
        Span {
            start: self.pos,
            end: self.pos.saturating_add(1).min(self.bytes.len()),
        }
    }

    fn slice(&self, start: usize, end: usize) -> &'src str {
        &self.source[start..end]
    }

    fn parse_text(&mut self) -> Result<(), ParseError<S::Error>> {
        let start = self.pos;

        while !self.eof() && self.current() != Some(b'<') {
            self.pos += 1;
        }

        if self.pos > start {
            let value = self.slice(start, self.pos);

            if value.bytes().any(|byte| !is_whitespace(byte)) {
                self.sink
                    .text(Text {
                        value,
                        span: Span {
                            start,
                            end: self.pos,
                        },
                    })
                    .map_err(ParseError::Sink)?;
            }
        }

        Ok(())
    }

    fn parse_tag(&mut self) -> Result<(), ParseError<S::Error>> {
        match self.current() {
            Some(b'?') => self.parse_processing_instruction(),

            Some(b'!') if self.starts_with(b"!--") => self.parse_comment(),

            Some(b'!') if self.starts_with(b"![CDATA[") => self.parse_cdata(),

            Some(b'!') if self.starts_with_ascii_case_insensitive(b"!DOCTYPE") => {
                self.parse_doctype()
            }

            Some(b'/') => {
                self.pos += 1;
                self.parse_closing_tag()
            }

            Some(byte) if is_name_char(byte) => self.parse_opening_tag(),

            Some(byte) => Err(ParseError::UnexpectedCharacter {
                byte: Some(byte),
                span: self.span_at_current(),
            }),

            None => Err(ParseError::UnexpectedEof {
                span: Span {
                    start: self.pos,
                    end: self.pos,
                },
            }),
        }
    }

    fn parse_opening_tag(&mut self) -> Result<(), ParseError<S::Error>> {
        let tag_start = self.pos.saturating_sub(1);
        let name_start = self.pos;
        let name = self.parse_name();
        let name_end = self.pos;

        if name.is_empty() {
            return Err(ParseError::ExpectedTagName {
                span: Span {
                    start: name_start,
                    end: name_start,
                },
            });
        }

        self.sink
            .start_element(StartElement {
                name,
                span: Span {
                    start: tag_start,
                    end: name_end,
                },
                name_span: Span {
                    start: name_start,
                    end: name_end,
                },
            })
            .map_err(ParseError::Sink)?;

        loop {
            self.skip_whitespace();

            match self.current() {
                Some(b'>') | Some(b'/') => break,

                Some(_) => {
                    let attr = self.parse_attribute()?;
                    self.sink.attribute(attr).map_err(ParseError::Sink)?;
                }

                None => {
                    return Err(ParseError::UnexpectedEof {
                        span: Span {
                            start: self.pos,
                            end: self.pos,
                        },
                    });
                }
            }
        }

        let self_closing = if self.current() == Some(b'/') {
            self.pos += 1;
            true
        } else {
            false
        };

        if self.current() != Some(b'>') {
            return Err(ParseError::ExpectedGt {
                span: self.span_at_current(),
            });
        }

        self.pos += 1;

        self.sink
            .finish_start_element(FinishStartElement {
                self_closing,
                span: Span {
                    start: tag_start,
                    end: self.pos,
                },
            })
            .map_err(ParseError::Sink)?;

        Ok(())
    }

    fn parse_closing_tag(&mut self) -> Result<(), ParseError<S::Error>> {
        let tag_start = self.pos.saturating_sub(2);
        let name_start = self.pos;
        let name = self.parse_name();
        let name_end = self.pos;

        if name.is_empty() {
            return Err(ParseError::ExpectedTagName {
                span: Span {
                    start: name_start,
                    end: name_start,
                },
            });
        }

        self.skip_whitespace();

        if self.current() != Some(b'>') {
            return Err(ParseError::ExpectedGt {
                span: self.span_at_current(),
            });
        }

        self.pos += 1;

        self.sink
            .end_element(EndElement {
                name,
                span: Span {
                    start: tag_start,
                    end: self.pos,
                },
                name_span: Span {
                    start: name_start,
                    end: name_end,
                },
            })
            .map_err(ParseError::Sink)?;

        Ok(())
    }

    fn parse_attribute(&mut self) -> Result<Attribute<'src>, ParseError<S::Error>> {
        let attr_start = self.pos;
        let name_start = self.pos;
        let name = self.parse_name();
        let name_end = self.pos;

        if name.is_empty() {
            return Err(ParseError::ExpectedAttributeName {
                span: Span {
                    start: name_start,
                    end: name_start,
                },
            });
        }

        self.skip_whitespace();

        let mut value = None;
        let mut value_span = None;
        let mut quote = None;

        if self.current() == Some(b'=') {
            self.pos += 1;
            self.skip_whitespace();

            let parsed = self.parse_attribute_value()?;

            value = Some(parsed.value);
            value_span = Some(parsed.span);
            quote = Some(parsed.quote);
        }

        Ok(Attribute {
            name,
            value,
            span: Span {
                start: attr_start,
                end: self.pos,
            },
            name_span: Span {
                start: name_start,
                end: name_end,
            },
            value_span,
            quote,
        })
    }

    fn parse_attribute_value(
        &mut self,
    ) -> Result<ParsedAttributeValue<'src>, ParseError<S::Error>> {
        match self.current() {
            Some(b'"') => self.parse_quoted_attribute_value(b'"', QuoteKind::Double),
            Some(b'\'') => self.parse_quoted_attribute_value(b'\'', QuoteKind::Single),
            Some(_) => Ok(self.parse_unquoted_attribute_value()),
            None => Err(ParseError::UnexpectedEof {
                span: Span {
                    start: self.pos,
                    end: self.pos,
                },
            }),
        }
    }

    fn parse_quoted_attribute_value(
        &mut self,
        quote_byte: u8,
        quote: QuoteKind,
    ) -> Result<ParsedAttributeValue<'src>, ParseError<S::Error>> {
        self.pos += 1;

        let value_start = self.pos;

        while let Some(byte) = self.current() {
            if byte == quote_byte {
                let value_end = self.pos;

                self.pos += 1;

                return Ok(ParsedAttributeValue {
                    value: self.slice(value_start, value_end),
                    span: Span {
                        start: value_start,
                        end: value_end,
                    },
                    quote,
                });
            }

            self.pos += 1;
        }

        Err(ParseError::UnclosedQuote {
            quote,
            span: Span {
                start: value_start,
                end: self.pos,
            },
        })
    }

    fn parse_unquoted_attribute_value(&mut self) -> ParsedAttributeValue<'src> {
        let value_start = self.pos;

        while let Some(byte) = self.current() {
            if is_whitespace(byte) || matches!(byte, b'>' | b'/') {
                break;
            }

            self.pos += 1;
        }

        ParsedAttributeValue {
            value: self.slice(value_start, self.pos),
            span: Span {
                start: value_start,
                end: self.pos,
            },
            quote: QuoteKind::None,
        }
    }

    fn parse_processing_instruction(&mut self) -> Result<(), ParseError<S::Error>> {
        let start = self.pos.saturating_sub(1);

        self.pos += 1;

        let value_start = self.pos;

        let Some(end_rel) = find_bytes(&self.bytes[self.pos..], b"?>") else {
            return Err(ParseError::ExpectedProcessingInstructionEnd {
                span: Span {
                    start,
                    end: self.pos,
                },
            });
        };

        let value_end = self.pos + end_rel;

        self.pos = value_end + 2;

        self.sink
            .processing_instruction(ProcessingInstruction {
                value: self.slice(value_start, value_end),
                span: Span {
                    start,
                    end: self.pos,
                },
            })
            .map_err(ParseError::Sink)?;

        Ok(())
    }

    fn parse_comment(&mut self) -> Result<(), ParseError<S::Error>> {
        let start = self.pos.saturating_sub(1);

        self.pos += 3;

        let value_start = self.pos;

        let Some(end_rel) = find_bytes(&self.bytes[self.pos..], b"-->") else {
            return Err(ParseError::ExpectedCommentEnd {
                span: Span {
                    start,
                    end: self.pos,
                },
            });
        };

        let value_end = self.pos + end_rel;

        self.pos = value_end + 3;

        self.sink
            .comment(Comment {
                value: self.slice(value_start, value_end),
                span: Span {
                    start,
                    end: self.pos,
                },
            })
            .map_err(ParseError::Sink)?;

        Ok(())
    }

    fn parse_cdata(&mut self) -> Result<(), ParseError<S::Error>> {
        let start = self.pos.saturating_sub(1);

        self.pos += b"![CDATA[".len();

        let value_start = self.pos;

        let Some(end_rel) = find_bytes(&self.bytes[self.pos..], b"]]>") else {
            return Err(ParseError::ExpectedCDataEnd {
                span: Span {
                    start,
                    end: self.pos,
                },
            });
        };

        let value_end = self.pos + end_rel;

        self.pos = value_end + 3;

        self.sink
            .cdata(CData {
                value: self.slice(value_start, value_end),
                span: Span {
                    start,
                    end: self.pos,
                },
            })
            .map_err(ParseError::Sink)?;

        Ok(())
    }

    fn parse_doctype(&mut self) -> Result<(), ParseError<S::Error>> {
        let start = self.pos.saturating_sub(1);

        self.pos += b"!DOCTYPE".len();

        let value_start = self.pos;
        let mut quote: Option<u8> = None;

        while let Some(byte) = self.current() {
            match (quote, byte) {
                (Some(open), current) if current == open => {
                    quote = None;
                }

                (None, b'"' | b'\'') => {
                    quote = Some(byte);
                }

                (None, b'>') => {
                    let value_end = self.pos;

                    self.pos += 1;

                    self.sink
                        .doctype(Doctype {
                            value: self.slice(value_start, value_end).trim(),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        })
                        .map_err(ParseError::Sink)?;

                    return Ok(());
                }

                _ => {}
            }

            self.pos += 1;
        }

        Err(ParseError::UnexpectedEof {
            span: Span {
                start,
                end: self.pos,
            },
        })
    }

    fn parse_name(&mut self) -> &'src str {
        let start = self.pos;

        while let Some(byte) = self.current() {
            if !is_name_char(byte) {
                break;
            }

            self.pos += 1;
        }

        self.slice(start, self.pos)
    }

    fn skip_whitespace(&mut self) {
        while self.current().is_some_and(is_whitespace) {
            self.pos += 1;
        }
    }

    fn starts_with_ascii_case_insensitive(&self, pattern: &[u8]) -> bool {
        let end = self.pos + pattern.len();

        if end > self.bytes.len() {
            return false;
        }

        self.bytes[self.pos..end].eq_ignore_ascii_case(pattern)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedAttributeValue<'src> {
    value: &'src str,
    span: Span,
    quote: QuoteKind,
}

#[inline]
pub fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-')
}

#[inline]
pub fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SvgDocument<'src> {
    pub children: Vec<SvgNode<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgNode<'src> {
    Element(SvgElement<'src>),
    Text(TextNode<'src>),
    Comment(CommentNode<'src>),
    CData(CDataNode<'src>),
    Doctype(DoctypeNode<'src>),
    ProcessingInstruction(ProcessingInstructionNode<'src>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgElement<'src> {
    pub name: &'src str,
    pub attributes: Vec<SvgAttribute<'src>>,
    pub children: Vec<SvgNode<'src>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgAttribute<'src> {
    pub name: &'src str,
    pub value: Option<&'src str>,
    pub span: Span,
    pub name_span: Span,
    pub value_span: Option<Span>,
    pub quote: Option<QuoteKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextNode<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentNode<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CDataNode<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctypeNode<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingInstructionNode<'src> {
    pub value: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeSinkError<'src> {
    NoCurrentElement {
        span: Span,
    },
    UnexpectedClosingTag {
        name: &'src str,
        span: Span,
    },
    MismatchedClosingTag {
        expected: &'src str,
        found: &'src str,
        span: Span,
    },
    UnclosedElements {
        names: Vec<&'src str>,
    },
}

impl fmt::Display for TreeSinkError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurrentElement { .. } => {
                write!(f, "no current element")
            }
            Self::UnexpectedClosingTag { name, .. } => {
                write!(f, "unexpected closing tag </{name}>")
            }
            Self::MismatchedClosingTag {
                expected, found, ..
            } => {
                write!(f, "expected closing tag </{expected}> but found </{found}>")
            }
            Self::UnclosedElements { names } => {
                write!(f, "unclosed elements: {}", names.join(", "))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TreeSink<'src> {
    document: SvgDocument<'src>,
    stack: Vec<ElementFrame<'src>>,
}

impl<'src> TreeSink<'src> {
    pub fn finish(self) -> SvgDocument<'src> {
        self.document
    }

    fn attach_node(&mut self, node: SvgNode<'src>) {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        } else {
            self.document.children.push(node);
        }
    }
}

impl<'src> SvgSink<'src> for TreeSink<'src> {
    type Error = TreeSinkError<'src>;

    fn start_element(&mut self, event: StartElement<'src>) -> Result<(), Self::Error> {
        self.stack.push(ElementFrame {
            name: event.name,
            attributes: Vec::new(),
            children: Vec::new(),
            span: event.span,
        });

        Ok(())
    }

    fn attribute(&mut self, attr: Attribute<'src>) -> Result<(), Self::Error> {
        let Some(current) = self.stack.last_mut() else {
            return Err(TreeSinkError::NoCurrentElement { span: attr.span });
        };

        current.attributes.push(SvgAttribute {
            name: attr.name,
            value: attr.value,
            span: attr.span,
            name_span: attr.name_span,
            value_span: attr.value_span,
            quote: attr.quote,
        });

        Ok(())
    }

    fn finish_start_element(&mut self, event: FinishStartElement) -> Result<(), Self::Error> {
        if event.self_closing {
            let Some(mut frame) = self.stack.pop() else {
                return Err(TreeSinkError::NoCurrentElement { span: event.span });
            };

            frame.span.end = event.span.end;

            self.attach_node(SvgNode::Element(frame.into_element()));
        } else if let Some(current) = self.stack.last_mut() {
            current.span.end = event.span.end;
        }

        Ok(())
    }

    fn end_element(&mut self, event: EndElement<'src>) -> Result<(), Self::Error> {
        let Some(mut frame) = self.stack.pop() else {
            return Err(TreeSinkError::UnexpectedClosingTag {
                name: event.name,
                span: event.span,
            });
        };

        if frame.name != event.name {
            return Err(TreeSinkError::MismatchedClosingTag {
                expected: frame.name,
                found: event.name,
                span: event.span,
            });
        }

        frame.span.end = event.span.end;

        self.attach_node(SvgNode::Element(frame.into_element()));

        Ok(())
    }

    fn text(&mut self, text: Text<'src>) -> Result<(), Self::Error> {
        self.attach_node(SvgNode::Text(TextNode {
            value: text.value,
            span: text.span,
        }));

        Ok(())
    }

    fn comment(&mut self, comment: Comment<'src>) -> Result<(), Self::Error> {
        self.attach_node(SvgNode::Comment(CommentNode {
            value: comment.value,
            span: comment.span,
        }));

        Ok(())
    }

    fn cdata(&mut self, cdata: CData<'src>) -> Result<(), Self::Error> {
        self.attach_node(SvgNode::CData(CDataNode {
            value: cdata.value,
            span: cdata.span,
        }));

        Ok(())
    }

    fn doctype(&mut self, doctype: Doctype<'src>) -> Result<(), Self::Error> {
        self.attach_node(SvgNode::Doctype(DoctypeNode {
            value: doctype.value,
            span: doctype.span,
        }));

        Ok(())
    }

    fn processing_instruction(
        &mut self,
        instruction: ProcessingInstruction<'src>,
    ) -> Result<(), Self::Error> {
        self.attach_node(SvgNode::ProcessingInstruction(ProcessingInstructionNode {
            value: instruction.value,
            span: instruction.span,
        }));

        Ok(())
    }

    fn finish_document(&mut self) -> Result<(), Self::Error> {
        if self.stack.is_empty() {
            return Ok(());
        }

        Err(TreeSinkError::UnclosedElements {
            names: self.stack.iter().map(|frame| frame.name).collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementFrame<'src> {
    name: &'src str,
    attributes: Vec<SvgAttribute<'src>>,
    children: Vec<SvgNode<'src>>,
    span: Span,
}

impl<'src> ElementFrame<'src> {
    fn into_element(self) -> SvgElement<'src> {
        SvgElement {
            name: self.name,
            attributes: self.attributes,
            children: self.children,
            span: self.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct LogSink<'src> {
        events: Vec<String>,
        _marker: std::marker::PhantomData<&'src ()>,
    }

    impl<'src> SvgSink<'src> for LogSink<'src> {
        type Error = std::convert::Infallible;

        fn start_element(&mut self, event: StartElement<'src>) -> Result<(), Self::Error> {
            self.events.push(format!("start:{}", event.name));
            Ok(())
        }

        fn attribute(&mut self, attr: Attribute<'src>) -> Result<(), Self::Error> {
            self.events
                .push(format!("attr:{}={:?}", attr.name, attr.value));
            Ok(())
        }

        fn finish_start_element(&mut self, event: FinishStartElement) -> Result<(), Self::Error> {
            self.events
                .push(format!("finish:self_closing={}", event.self_closing));
            Ok(())
        }

        fn end_element(&mut self, event: EndElement<'src>) -> Result<(), Self::Error> {
            self.events.push(format!("end:{}", event.name));
            Ok(())
        }

        fn text(&mut self, text: Text<'src>) -> Result<(), Self::Error> {
            self.events.push(format!("text:{}", text.value));
            Ok(())
        }
    }

    #[test]
    fn emits_basic_events() {
        let mut sink = LogSink::default();

        parse_with_sink(
            r#"<svg viewBox="0 0 24 24"><path fill='none'/></svg>"#,
            &mut sink,
        )
        .unwrap();

        assert_eq!(
            sink.events,
            vec![
                "start:svg",
                "attr:viewBox=Some(\"0 0 24 24\")",
                "finish:self_closing=false",
                "start:path",
                "attr:fill=Some(\"none\")",
                "finish:self_closing=true",
                "end:svg",
            ]
        );
    }

    #[test]
    fn builds_tree() {
        let doc = parse(r#"<svg viewBox="0 0 24 24"><path fill="none" /></svg>"#).unwrap();

        let SvgNode::Element(svg) = &doc.children[0] else {
            panic!("expected svg element");
        };

        assert_eq!(svg.name, "svg");
        assert_eq!(svg.attributes[0].name, "viewBox");
        assert_eq!(svg.attributes[0].value, Some("0 0 24 24"));
        assert_eq!(svg.children.len(), 1);

        let SvgNode::Element(path) = &svg.children[0] else {
            panic!("expected path element");
        };

        assert_eq!(path.name, "path");
        assert_eq!(path.attributes[0].name, "fill");
        assert_eq!(path.attributes[0].value, Some("none"));
    }

    #[test]
    fn detects_mismatched_closing_tag() {
        let err = parse("<svg><g></svg>").unwrap_err();

        assert!(matches!(
            err,
            ParseError::Sink(TreeSinkError::MismatchedClosingTag {
                expected: "g",
                found: "svg",
                ..
            })
        ));
    }

    #[test]
    fn parses_cdata() {
        let doc = parse("<svg><![CDATA[hello <world>]]></svg>").unwrap();

        let SvgNode::Element(svg) = &doc.children[0] else {
            panic!("expected svg");
        };

        let SvgNode::CData(cdata) = &svg.children[0] else {
            panic!("expected cdata");
        };

        assert_eq!(cdata.value, "hello <world>");
    }
}
