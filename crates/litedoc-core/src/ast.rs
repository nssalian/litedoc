//! Abstract Syntax Tree types for LiteDoc documents.
//!
//! This module contains all the AST node types produced by the parser.
//! The AST is designed to be:
//!
//! - **Zero-copy**: Uses `Cow<'a, str>` to borrow from input when possible
//! - **Span-tracked**: Every node includes source location information
//! - **Comprehensive**: Supports all LiteDoc and Markdown constructs

use crate::span::Span;

/// Parsing profile that determines syntax rules.
///
/// The profile affects how the parser interprets certain constructs
/// and which features are enabled by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Full LiteDoc syntax with explicit fencing.
    ///
    /// This is the native format optimized for AI consumption.
    Litedoc,
    /// CommonMark with GFM extensions (tables, strikethrough, autolinks).
    Md,
    /// Strict CommonMark compliance only.
    MdStrict,
}

/// Optional modules that extend parser capabilities.
///
/// Modules can be enabled via the `@modules` directive or parser configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    /// GFM-style tables with `|` delimiters.
    Tables,
    /// Footnote definitions and references.
    Footnotes,
    /// LaTeX-style math blocks.
    Math,
    /// Task list items with `[ ]` and `[x]` checkboxes.
    Tasks,
    /// `~~strikethrough~~` syntax.
    Strikethrough,
    /// Automatic URL detection and linking.
    Autolink,
    /// Raw HTML pass-through blocks.
    Html,
}

/// A parsed LiteDoc document.
///
/// The document is the root of the AST and contains all parsed content.
/// It preserves the parsing profile, enabled modules, optional metadata,
/// and all content blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Document<'a> {
    /// The parsing profile used (may differ from parser default if `@profile` directive present).
    pub profile: Profile,
    /// Enabled modules from `@modules` directive.
    pub modules: Vec<Module>,
    /// Optional metadata from `--- meta` block.
    pub metadata: Option<Metadata<'a>>,
    /// Content blocks in document order.
    pub blocks: Vec<Block<'a>>,
    /// Source span covering the entire document.
    pub span: Span,
}

/// Document metadata from the `--- meta` block.
///
/// Metadata provides key-value pairs for document properties like
/// title, author, date, tags, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata<'a> {
    /// Key-value entries in declaration order.
    pub entries: Vec<(CowStr<'a>, AttrValue<'a>)>,
    /// Source span of the metadata block.
    pub span: Span,
}

/// Typed attribute values for metadata entries.
///
/// Values are automatically parsed into appropriate types:
/// - Quoted strings → `Str`
/// - `true`/`false` → `Bool`
/// - Integers → `Int`
/// - Decimals → `Float`
/// - `[a, b, c]` → `List`
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue<'a> {
    /// String value (quotes stripped).
    Str(CowStr<'a>),
    /// Boolean value.
    Bool(bool),
    /// 64-bit signed integer.
    Int(i64),
    /// 64-bit floating point.
    Float(f64),
    /// Nested list of values.
    List(Vec<AttrValue<'a>>),
}

/// Block-level AST nodes.
///
/// Blocks are the primary structural elements of a document.
/// Each variant represents a distinct block type with its own structure.
#[derive(Debug, Clone, PartialEq)]
pub enum Block<'a> {
    /// Section heading (levels 1-6).
    Heading(Heading<'a>),
    /// Text paragraph with inline formatting.
    Paragraph(Paragraph<'a>),
    /// Ordered or unordered list.
    List(List<'a>),
    /// Fenced code block with optional language.
    CodeBlock(CodeBlock<'a>),
    /// Callout/admonition block (note, warning, etc.).
    Callout(Callout<'a>),
    /// Block quotation.
    Quote(Quote<'a>),
    /// Figure with image and caption.
    Figure(Figure<'a>),
    /// Data table with rows and cells.
    Table(Table<'a>),
    /// Footnote definitions.
    Footnotes(Footnotes<'a>),
    /// Mathematical equation (inline or display).
    Math(MathBlock<'a>),
    /// Horizontal rule / thematic break.
    ThematicBreak(Span),
    /// Raw HTML content (when HTML module enabled).
    Html(HtmlBlock<'a>),
    /// Unparsed/unknown block content (error recovery).
    Raw(RawBlock<'a>),
}

/// Section heading with level and inline content.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading<'a> {
    /// Heading level (1-6).
    pub level: u8,
    /// Inline content (may include formatting).
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// Text paragraph containing inline elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph<'a> {
    /// Inline content with formatting.
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// List ordering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// Numbered list (1. 2. 3.).
    Ordered,
    /// Bulleted list (- or *).
    Unordered,
}

/// A list block containing multiple items.
#[derive(Debug, Clone, PartialEq)]
pub struct List<'a> {
    /// Ordered or unordered.
    pub kind: ListKind,
    /// Starting number for ordered lists.
    pub start: Option<u64>,
    /// List items.
    pub items: Vec<ListItem<'a>>,
    /// Source span.
    pub span: Span,
}

/// A single list item (may contain nested blocks).
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem<'a> {
    /// Content blocks within the item.
    pub blocks: Vec<Block<'a>>,
    /// Source span.
    pub span: Span,
}

/// Fenced code block with syntax highlighting hint.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock<'a> {
    /// Language identifier (e.g., "rust", "python").
    pub lang: CowStr<'a>,
    /// Raw code content.
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Callout/admonition block for notes, warnings, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Callout<'a> {
    /// Callout type (note, warning, info, tip, etc.).
    pub kind: CowStr<'a>,
    /// Optional title override.
    pub title: Option<CowStr<'a>>,
    /// Content blocks.
    pub blocks: Vec<Block<'a>>,
    /// Source span.
    pub span: Span,
}

/// Block quotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote<'a> {
    /// Quoted content blocks.
    pub blocks: Vec<Block<'a>>,
    /// Source span.
    pub span: Span,
}

/// Figure with image and optional caption.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure<'a> {
    /// Image source URL or path.
    pub src: CowStr<'a>,
    /// Alt text for accessibility.
    pub alt: CowStr<'a>,
    /// Optional figure caption.
    pub caption: Option<CowStr<'a>>,
    /// Source span.
    pub span: Span,
}

/// Data table with header and body rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Table<'a> {
    /// All table rows (first may be header).
    pub rows: Vec<TableRow<'a>>,
    /// Source span.
    pub span: Span,
}

/// A single table row.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow<'a> {
    /// Cells in this row.
    pub cells: Vec<TableCell<'a>>,
    /// Whether this is a header row.
    pub header: bool,
    /// Source span.
    pub span: Span,
}

/// A single table cell.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell<'a> {
    /// Cell content (inline elements).
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// Container for footnote definitions.
#[derive(Debug, Clone, PartialEq)]
pub struct Footnotes<'a> {
    /// Footnote definitions.
    pub defs: Vec<FootnoteDef<'a>>,
    /// Source span.
    pub span: Span,
}

/// A single footnote definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteDef<'a> {
    /// Footnote label (e.g., "1", "note").
    pub label: CowStr<'a>,
    /// Footnote content blocks.
    pub blocks: Vec<Block<'a>>,
    /// Source span.
    pub span: Span,
}

/// Mathematical equation block (LaTeX).
#[derive(Debug, Clone, PartialEq)]
pub struct MathBlock<'a> {
    /// Whether this is display math (vs inline).
    pub display: bool,
    /// LaTeX content.
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Raw HTML block content.
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlBlock<'a> {
    /// Raw HTML content.
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Unparsed block content (for error recovery).
#[derive(Debug, Clone, PartialEq)]
pub struct RawBlock<'a> {
    /// Raw unparsed content.
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Inline-level AST nodes (within paragraphs, headings, etc.).
///
/// Inline elements represent text-level formatting and can be nested.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline<'a> {
    /// Plain text content.
    Text(Text<'a>),
    /// Emphasized text (*italic*).
    Emphasis(Emphasis<'a>),
    /// Strong text (**bold**).
    Strong(Strong<'a>),
    /// Inline code (`code`).
    CodeSpan(CodeSpan<'a>),
    /// Hyperlink with label and URL.
    Link(Link<'a>),
    /// Auto-detected URL link.
    AutoLink(AutoLink<'a>),
    /// Strikethrough text (~~deleted~~).
    Strikethrough(Strikethrough<'a>),
    /// Footnote reference ([^label]).
    FootnoteRef(FootnoteRef<'a>),
    /// Hard line break (explicit).
    HardBreak(Span),
    /// Soft line break (newline in source).
    SoftBreak(Span),
}

/// Plain text content.
#[derive(Debug, Clone, PartialEq)]
pub struct Text<'a> {
    /// The text content.
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Emphasized (italic) text.
#[derive(Debug, Clone, PartialEq)]
pub struct Emphasis<'a> {
    /// Nested inline content.
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// Strong (bold) text.
#[derive(Debug, Clone, PartialEq)]
pub struct Strong<'a> {
    /// Nested inline content.
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// Strikethrough text.
#[derive(Debug, Clone, PartialEq)]
pub struct Strikethrough<'a> {
    /// Nested inline content.
    pub content: Vec<Inline<'a>>,
    /// Source span.
    pub span: Span,
}

/// Inline code span.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeSpan<'a> {
    /// Code content (not parsed for formatting).
    pub content: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Hyperlink with label and destination.
#[derive(Debug, Clone, PartialEq)]
pub struct Link<'a> {
    /// Link text (may contain nested formatting).
    pub label: Vec<Inline<'a>>,
    /// Link destination URL.
    pub url: CowStr<'a>,
    /// Optional title (for tooltips).
    pub title: Option<CowStr<'a>>,
    /// Source span.
    pub span: Span,
}

/// Automatically detected URL.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoLink<'a> {
    /// The URL.
    pub url: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Reference to a footnote.
#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteRef<'a> {
    /// Footnote label being referenced.
    pub label: CowStr<'a>,
    /// Source span.
    pub span: Span,
}

/// Borrowed or owned string type for zero-copy parsing.
pub type CowStr<'a> = std::borrow::Cow<'a, str>;
