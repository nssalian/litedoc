//! Python bindings for LiteDoc parser.

use litedoc_core::{
    ast::{AttrValue, Block, Document, Inline, Metadata, Module},
    error::{ParseError as CoreParseError, ParseErrorKind as CoreParseErrorKind},
    span::Span as CoreSpan,
    ParseResult as CoreParseResult, Parser as CoreParser, Profile as CoreProfile,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;

// ============================================================================
// Span
// ============================================================================

/// Source location in the input text (byte offsets).
#[pyclass(frozen, get_all, name = "Span")]
#[derive(Clone)]
pub struct PySpan {
    pub start: u32,
    pub end: u32,
}

#[pymethods]
impl PySpan {
    fn __repr__(&self) -> String {
        format!("Span({}, {})", self.start, self.end)
    }

    #[getter]
    fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

impl From<CoreSpan> for PySpan {
    fn from(s: CoreSpan) -> Self {
        PySpan {
            start: s.start,
            end: s.end,
        }
    }
}

// ============================================================================
// Enums
// ============================================================================

/// Parsing profile.
#[pyclass(frozen, eq, eq_int, name = "Profile")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyProfile {
    Litedoc,
    Md,
    MdStrict,
}

impl From<CoreProfile> for PyProfile {
    fn from(p: CoreProfile) -> Self {
        match p {
            CoreProfile::Litedoc => PyProfile::Litedoc,
            CoreProfile::Md => PyProfile::Md,
            CoreProfile::MdStrict => PyProfile::MdStrict,
        }
    }
}

impl From<PyProfile> for CoreProfile {
    fn from(p: PyProfile) -> Self {
        match p {
            PyProfile::Litedoc => CoreProfile::Litedoc,
            PyProfile::Md => CoreProfile::Md,
            PyProfile::MdStrict => CoreProfile::MdStrict,
        }
    }
}

/// Parser module.
#[pyclass(frozen, eq, eq_int, name = "ModuleKind")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyModuleKind {
    Tables,
    Footnotes,
    Math,
    Tasks,
    Strikethrough,
    Autolink,
    Html,
}

impl From<Module> for PyModuleKind {
    fn from(m: Module) -> Self {
        match m {
            Module::Tables => PyModuleKind::Tables,
            Module::Footnotes => PyModuleKind::Footnotes,
            Module::Math => PyModuleKind::Math,
            Module::Tasks => PyModuleKind::Tasks,
            Module::Strikethrough => PyModuleKind::Strikethrough,
            Module::Autolink => PyModuleKind::Autolink,
            Module::Html => PyModuleKind::Html,
        }
    }
}

/// Parse error category.
#[pyclass(frozen, eq, eq_int, name = "ParseErrorKind")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyParseErrorKind {
    UnexpectedEof,
    UnclosedDelimiter,
    InvalidSyntax,
    UnknownDirective,
    InvalidMetadata,
    Other,
}

impl From<CoreParseErrorKind> for PyParseErrorKind {
    fn from(k: CoreParseErrorKind) -> Self {
        match k {
            CoreParseErrorKind::UnexpectedEof => PyParseErrorKind::UnexpectedEof,
            CoreParseErrorKind::UnclosedDelimiter => PyParseErrorKind::UnclosedDelimiter,
            CoreParseErrorKind::InvalidSyntax => PyParseErrorKind::InvalidSyntax,
            CoreParseErrorKind::UnknownDirective => PyParseErrorKind::UnknownDirective,
            CoreParseErrorKind::InvalidMetadata => PyParseErrorKind::InvalidMetadata,
            CoreParseErrorKind::Other => PyParseErrorKind::Other,
        }
    }
}

/// A parse error.
#[pyclass(frozen, get_all, name = "ParseError")]
#[derive(Clone)]
pub struct PyParseError {
    pub message: String,
    pub span: Option<PySpan>,
    pub kind: PyParseErrorKind,
    pub recoverable: bool,
}

#[pymethods]
impl PyParseError {
    fn __repr__(&self) -> String {
        format!("ParseError({:?}, {:?})", self.message, self.kind)
    }

    fn __str__(&self) -> String {
        match &self.span {
            Some(s) => format!("{} at bytes {}..{}", self.message, s.start, s.end),
            None => self.message.clone(),
        }
    }
}

impl From<CoreParseError> for PyParseError {
    fn from(e: CoreParseError) -> Self {
        PyParseError {
            message: e.message,
            span: e.span.map(PySpan::from),
            kind: e.kind.into(),
            recoverable: e.recoverable,
        }
    }
}

// ============================================================================
// Block types
// ============================================================================

/// Section heading.
#[pyclass(frozen, get_all, name = "Heading")]
pub struct PyHeading {
    pub level: u8,
    pub content: PyObject,
    pub span: PySpan,
}

/// Text paragraph.
#[pyclass(frozen, get_all, name = "Paragraph")]
pub struct PyParagraph {
    pub content: PyObject,
    pub span: PySpan,
}

/// List kind.
#[pyclass(frozen, eq, eq_int, name = "ListKind")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyListKind {
    Ordered,
    Unordered,
}

/// List item.
#[pyclass(frozen, get_all, name = "ListItem")]
pub struct PyListItem {
    pub blocks: PyObject,
    pub span: PySpan,
}

/// Ordered or unordered list.
#[pyclass(frozen, get_all, name = "List")]
pub struct PyList_ {
    pub kind: PyListKind,
    pub start: Option<u64>,
    pub items: PyObject,
    pub span: PySpan,
}

/// Fenced code block.
#[pyclass(frozen, get_all, name = "CodeBlock")]
pub struct PyCodeBlock {
    pub lang: String,
    pub content: String,
    pub span: PySpan,
}

/// Callout/admonition.
#[pyclass(frozen, get_all, name = "Callout")]
pub struct PyCallout {
    pub kind: String,
    pub title: Option<String>,
    pub blocks: PyObject,
    pub span: PySpan,
}

/// Block quote.
#[pyclass(frozen, get_all, name = "Quote")]
pub struct PyQuote {
    pub blocks: PyObject,
    pub span: PySpan,
}

/// Figure with image.
#[pyclass(frozen, get_all, name = "Figure")]
pub struct PyFigure {
    pub src: String,
    pub alt: String,
    pub caption: Option<String>,
    pub span: PySpan,
}

/// Table cell.
#[pyclass(frozen, get_all, name = "TableCell")]
pub struct PyTableCell {
    pub content: PyObject,
    pub span: PySpan,
}

/// Table row.
#[pyclass(frozen, get_all, name = "TableRow")]
pub struct PyTableRow {
    pub cells: PyObject,
    pub header: bool,
    pub span: PySpan,
}

/// Data table.
#[pyclass(frozen, get_all, name = "Table")]
pub struct PyTable {
    pub rows: PyObject,
    pub span: PySpan,
}

/// Footnote definition.
#[pyclass(frozen, get_all, name = "FootnoteDef")]
pub struct PyFootnoteDef {
    pub label: String,
    pub blocks: PyObject,
    pub span: PySpan,
}

/// Footnotes container.
#[pyclass(frozen, get_all, name = "Footnotes")]
pub struct PyFootnotes {
    pub defs: PyObject,
    pub span: PySpan,
}

/// Math block.
#[pyclass(frozen, get_all, name = "MathBlock")]
pub struct PyMathBlock {
    pub display: bool,
    pub content: String,
    pub span: PySpan,
}

/// Thematic break (horizontal rule).
#[pyclass(frozen, get_all, name = "ThematicBreak")]
pub struct PyThematicBreak {
    pub span: PySpan,
}

/// Raw HTML block.
#[pyclass(frozen, get_all, name = "HtmlBlock")]
pub struct PyHtmlBlock {
    pub content: String,
    pub span: PySpan,
}

/// Unparsed block (error recovery).
#[pyclass(frozen, get_all, name = "RawBlock")]
pub struct PyRawBlock {
    pub content: String,
    pub span: PySpan,
}

// ============================================================================
// Inline types
// ============================================================================

/// Plain text.
#[pyclass(frozen, get_all, name = "Text")]
pub struct PyText {
    pub content: String,
    pub span: PySpan,
}

/// Emphasis (italic).
#[pyclass(frozen, get_all, name = "Emphasis")]
pub struct PyEmphasis {
    pub content: PyObject,
    pub span: PySpan,
}

/// Strong (bold).
#[pyclass(frozen, get_all, name = "Strong")]
pub struct PyStrong {
    pub content: PyObject,
    pub span: PySpan,
}

/// Strikethrough.
#[pyclass(frozen, get_all, name = "Strikethrough")]
pub struct PyStrikethrough {
    pub content: PyObject,
    pub span: PySpan,
}

/// Inline code.
#[pyclass(frozen, get_all, name = "CodeSpan")]
pub struct PyCodeSpan {
    pub content: String,
    pub span: PySpan,
}

/// Hyperlink.
#[pyclass(frozen, get_all, name = "Link")]
pub struct PyLink {
    pub label: PyObject,
    pub url: String,
    pub title: Option<String>,
    pub span: PySpan,
}

/// Auto-detected URL.
#[pyclass(frozen, get_all, name = "AutoLink")]
pub struct PyAutoLink {
    pub url: String,
    pub span: PySpan,
}

/// Footnote reference.
#[pyclass(frozen, get_all, name = "FootnoteRef")]
pub struct PyFootnoteRef {
    pub label: String,
    pub span: PySpan,
}

/// Hard line break.
#[pyclass(frozen, get_all, name = "HardBreak")]
pub struct PyHardBreak {
    pub span: PySpan,
}

/// Soft line break.
#[pyclass(frozen, get_all, name = "SoftBreak")]
pub struct PySoftBreak {
    pub span: PySpan,
}

// ============================================================================
// Conversion
// ============================================================================

fn convert_inlines(py: Python<'_>, inlines: Vec<Inline>) -> PyObject {
    let list = PyList::empty(py);
    for inline in inlines {
        list.append(convert_inline(py, inline)).unwrap();
    }
    list.into()
}

fn convert_inline(py: Python<'_>, inline: Inline) -> PyObject {
    match inline {
        Inline::Text(t) => Py::new(
            py,
            PyText {
                content: t.content.into_owned(),
                span: t.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::Emphasis(e) => Py::new(
            py,
            PyEmphasis {
                content: convert_inlines(py, e.content),
                span: e.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::Strong(s) => Py::new(
            py,
            PyStrong {
                content: convert_inlines(py, s.content),
                span: s.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::Strikethrough(s) => Py::new(
            py,
            PyStrikethrough {
                content: convert_inlines(py, s.content),
                span: s.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::CodeSpan(c) => Py::new(
            py,
            PyCodeSpan {
                content: c.content.into_owned(),
                span: c.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::Link(l) => Py::new(
            py,
            PyLink {
                label: convert_inlines(py, l.label),
                url: l.url.into_owned(),
                title: l.title.map(|t| t.into_owned()),
                span: l.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::AutoLink(a) => Py::new(
            py,
            PyAutoLink {
                url: a.url.into_owned(),
                span: a.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::FootnoteRef(f) => Py::new(
            py,
            PyFootnoteRef {
                label: f.label.into_owned(),
                span: f.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Inline::HardBreak(span) => Py::new(py, PyHardBreak { span: span.into() })
            .unwrap()
            .into_any(),
        Inline::SoftBreak(span) => Py::new(py, PySoftBreak { span: span.into() })
            .unwrap()
            .into_any(),
    }
}

fn convert_blocks(py: Python<'_>, blocks: Vec<Block>) -> PyObject {
    let list = PyList::empty(py);
    for block in blocks {
        list.append(convert_block(py, block)).unwrap();
    }
    list.into()
}

fn convert_block(py: Python<'_>, block: Block) -> PyObject {
    use litedoc_core::ast::ListKind as CoreListKind;
    match block {
        Block::Heading(h) => Py::new(
            py,
            PyHeading {
                level: h.level,
                content: convert_inlines(py, h.content),
                span: h.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Paragraph(p) => Py::new(
            py,
            PyParagraph {
                content: convert_inlines(py, p.content),
                span: p.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::List(l) => {
            let items = PyList::empty(py);
            for item in l.items {
                let li = Py::new(
                    py,
                    PyListItem {
                        blocks: convert_blocks(py, item.blocks),
                        span: item.span.into(),
                    },
                )
                .unwrap();
                items.append(li).unwrap();
            }
            Py::new(
                py,
                PyList_ {
                    kind: match l.kind {
                        CoreListKind::Ordered => PyListKind::Ordered,
                        CoreListKind::Unordered => PyListKind::Unordered,
                    },
                    start: l.start,
                    items: items.into(),
                    span: l.span.into(),
                },
            )
            .unwrap()
            .into_any()
        }
        Block::CodeBlock(c) => Py::new(
            py,
            PyCodeBlock {
                lang: c.lang.into_owned(),
                content: c.content.into_owned(),
                span: c.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Callout(c) => Py::new(
            py,
            PyCallout {
                kind: c.kind.into_owned(),
                title: c.title.map(|t| t.into_owned()),
                blocks: convert_blocks(py, c.blocks),
                span: c.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Quote(q) => Py::new(
            py,
            PyQuote {
                blocks: convert_blocks(py, q.blocks),
                span: q.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Figure(f) => Py::new(
            py,
            PyFigure {
                src: f.src.into_owned(),
                alt: f.alt.into_owned(),
                caption: f.caption.map(|c| c.into_owned()),
                span: f.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Table(t) => {
            let rows = PyList::empty(py);
            for row in t.rows {
                let cells = PyList::empty(py);
                for cell in row.cells {
                    let tc = Py::new(
                        py,
                        PyTableCell {
                            content: convert_inlines(py, cell.content),
                            span: cell.span.into(),
                        },
                    )
                    .unwrap();
                    cells.append(tc).unwrap();
                }
                let tr = Py::new(
                    py,
                    PyTableRow {
                        cells: cells.into(),
                        header: row.header,
                        span: row.span.into(),
                    },
                )
                .unwrap();
                rows.append(tr).unwrap();
            }
            Py::new(
                py,
                PyTable {
                    rows: rows.into(),
                    span: t.span.into(),
                },
            )
            .unwrap()
            .into_any()
        }
        Block::Footnotes(f) => {
            let defs = PyList::empty(py);
            for def in f.defs {
                let fd = Py::new(
                    py,
                    PyFootnoteDef {
                        label: def.label.into_owned(),
                        blocks: convert_blocks(py, def.blocks),
                        span: def.span.into(),
                    },
                )
                .unwrap();
                defs.append(fd).unwrap();
            }
            Py::new(
                py,
                PyFootnotes {
                    defs: defs.into(),
                    span: f.span.into(),
                },
            )
            .unwrap()
            .into_any()
        }
        Block::Math(m) => Py::new(
            py,
            PyMathBlock {
                display: m.display,
                content: m.content.into_owned(),
                span: m.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::ThematicBreak(span) => Py::new(py, PyThematicBreak { span: span.into() })
            .unwrap()
            .into_any(),
        Block::Html(h) => Py::new(
            py,
            PyHtmlBlock {
                content: h.content.into_owned(),
                span: h.span.into(),
            },
        )
        .unwrap()
        .into_any(),
        Block::Raw(r) => Py::new(
            py,
            PyRawBlock {
                content: r.content.into_owned(),
                span: r.span.into(),
            },
        )
        .unwrap()
        .into_any(),
    }
}

fn convert_attr_value(py: Python<'_>, v: AttrValue) -> PyObject {
    match v {
        AttrValue::Str(s) => s.into_owned().into_py_any(py).unwrap(),
        AttrValue::Bool(b) => b.into_py_any(py).unwrap(),
        AttrValue::Int(i) => i.into_py_any(py).unwrap(),
        AttrValue::Float(f) => f.into_py_any(py).unwrap(),
        AttrValue::List(l) => {
            let list = PyList::empty(py);
            for item in l {
                list.append(convert_attr_value(py, item)).unwrap();
            }
            list.into()
        }
    }
}

// ============================================================================
// PyDocument
// ============================================================================

/// A parsed LiteDoc document.
#[pyclass(frozen, name = "Document")]
pub struct PyDocument {
    #[pyo3(get)]
    pub profile: PyProfile,
    #[pyo3(get)]
    pub modules: Vec<PyModuleKind>,
    #[pyo3(get)]
    pub metadata: Option<PyObject>,
    #[pyo3(get)]
    pub blocks: PyObject,
    #[pyo3(get)]
    pub span: PySpan,
}

#[pymethods]
impl PyDocument {
    fn __repr__(&self, py: Python<'_>) -> String {
        let blocks: &Bound<'_, PyList> = self.blocks.downcast_bound(py).unwrap();
        format!(
            "Document(profile={:?}, blocks={}, metadata={})",
            self.profile,
            blocks.len(),
            self.metadata.is_some()
        )
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        let blocks: &Bound<'_, PyList> = self.blocks.downcast_bound(py).unwrap();
        blocks.len()
    }
}

fn convert_document(py: Python<'_>, doc: Document) -> PyDocument {
    let metadata = doc.metadata.map(|Metadata { entries, span: _ }| {
        let dict = PyDict::new(py);
        for (k, v) in entries {
            dict.set_item(k.into_owned(), convert_attr_value(py, v))
                .unwrap();
        }
        dict.into()
    });

    PyDocument {
        profile: doc.profile.into(),
        modules: doc.modules.into_iter().map(PyModuleKind::from).collect(),
        metadata,
        blocks: convert_blocks(py, doc.blocks),
        span: doc.span.into(),
    }
}

// ============================================================================
// ParseResult
// ============================================================================

/// Result of parsing with error recovery.
#[pyclass(frozen, name = "ParseResult")]
pub struct PyParseResult {
    #[pyo3(get)]
    pub document: Py<PyDocument>,
    #[pyo3(get)]
    pub errors: Vec<PyParseError>,
}

#[pymethods]
impl PyParseResult {
    #[getter]
    fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let doc = self.document.borrow(py);
        let blocks: &Bound<'_, PyList> = doc.blocks.downcast_bound(py).unwrap();
        format!(
            "ParseResult(ok={}, blocks={}, errors={})",
            self.errors.is_empty(),
            blocks.len(),
            self.errors.len()
        )
    }
}

// ============================================================================
// Parser
// ============================================================================

/// LiteDoc parser.
///
/// Args:
///     profile: Profile.Litedoc (default), Profile.Md, or Profile.MdStrict
#[pyclass(name = "Parser")]
pub struct PyParser {
    profile: CoreProfile,
}

#[pymethods]
impl PyParser {
    #[new]
    #[pyo3(signature = (profile=None), text_signature = "(profile=None)")]
    fn new(profile: Option<PyProfile>) -> Self {
        PyParser {
            profile: profile.unwrap_or(PyProfile::Litedoc).into(),
        }
    }

    /// Parse a LiteDoc string. Raises ValueError on error.
    #[pyo3(text_signature = "(self, input)")]
    fn parse(&self, py: Python<'_>, input: &str) -> PyResult<PyDocument> {
        let mut parser = CoreParser::new(self.profile);
        match parser.parse(input) {
            Ok(doc) => Ok(convert_document(py, doc)),
            Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        }
    }

    /// Parse with error recovery. Always returns a result.
    #[pyo3(text_signature = "(self, input)")]
    fn parse_with_recovery(&self, py: Python<'_>, input: &str) -> PyParseResult {
        let mut parser = CoreParser::new(self.profile);
        let CoreParseResult { document, errors } = parser.parse_with_recovery(input);
        PyParseResult {
            document: Py::new(py, convert_document(py, document)).unwrap(),
            errors: errors.into_iter().map(PyParseError::from).collect(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Parser(profile={:?})", self.profile)
    }
}

// ============================================================================
// Module functions
// ============================================================================

/// Parse a LiteDoc string.
///
/// Args:
///     input: Document string to parse
///     profile: Parsing profile (default: Profile.Litedoc)
///
/// Returns:
///     Document: Parsed document
///
/// Raises:
///     ValueError: On parse error
#[pyfunction]
#[pyo3(signature = (input, profile=None), text_signature = "(input, profile=None)")]
fn parse(py: Python<'_>, input: &str, profile: Option<PyProfile>) -> PyResult<PyDocument> {
    let p = PyParser::new(profile);
    p.parse(py, input)
}

/// Parse with error recovery. Always returns a result.
///
/// Args:
///     input: Document string
///     profile: Parsing profile (default: Profile.Litedoc)
///
/// Returns:
///     ParseResult: Result with document and errors
#[pyfunction]
#[pyo3(signature = (input, profile=None), text_signature = "(input, profile=None)")]
fn parse_with_recovery(py: Python<'_>, input: &str, profile: Option<PyProfile>) -> PyParseResult {
    let p = PyParser::new(profile);
    p.parse_with_recovery(py, input)
}

// ============================================================================
// Module
// ============================================================================

/// LiteDoc - Deterministic document parser for AI agents.
#[pymodule]
fn pyld(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySpan>()?;
    m.add_class::<PyProfile>()?;
    m.add_class::<PyModuleKind>()?;
    m.add_class::<PyParser>()?;
    m.add_class::<PyDocument>()?;
    m.add_class::<PyParseResult>()?;
    m.add_class::<PyParseErrorKind>()?;
    m.add_class::<PyParseError>()?;
    m.add_class::<PyHeading>()?;
    m.add_class::<PyParagraph>()?;
    m.add_class::<PyList_>()?;
    m.add_class::<PyListItem>()?;
    m.add_class::<PyListKind>()?;
    m.add_class::<PyCodeBlock>()?;
    m.add_class::<PyCallout>()?;
    m.add_class::<PyQuote>()?;
    m.add_class::<PyFigure>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyTableRow>()?;
    m.add_class::<PyTableCell>()?;
    m.add_class::<PyFootnotes>()?;
    m.add_class::<PyFootnoteDef>()?;
    m.add_class::<PyMathBlock>()?;
    m.add_class::<PyThematicBreak>()?;
    m.add_class::<PyHtmlBlock>()?;
    m.add_class::<PyRawBlock>()?;
    m.add_class::<PyText>()?;
    m.add_class::<PyEmphasis>()?;
    m.add_class::<PyStrong>()?;
    m.add_class::<PyStrikethrough>()?;
    m.add_class::<PyCodeSpan>()?;
    m.add_class::<PyLink>()?;
    m.add_class::<PyAutoLink>()?;
    m.add_class::<PyFootnoteRef>()?;
    m.add_class::<PyHardBreak>()?;
    m.add_class::<PySoftBreak>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_with_recovery, m)?)?;
    Ok(())
}
