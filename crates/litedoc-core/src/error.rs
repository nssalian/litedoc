use crate::span::Span;
use std::fmt;

/// Error kinds for categorizing parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Unexpected end of input
    UnexpectedEof,
    /// Unclosed delimiter (code block, fenced block, etc.)
    UnclosedDelimiter,
    /// Invalid syntax that couldn't be parsed
    InvalidSyntax,
    /// Unknown directive or block type
    UnknownDirective,
    /// Malformed metadata
    InvalidMetadata,
    /// Generic parse error
    Other,
}

/// A parse error with location and recovery information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable error message
    pub message: String,
    /// Source location where the error occurred
    pub span: Option<Span>,
    /// Error categorization
    pub kind: ParseErrorKind,
    /// Whether parsing can continue after this error
    pub recoverable: bool,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
            kind: ParseErrorKind::Other,
            recoverable: true,
        }
    }

    /// Create an error for unexpected end of input.
    pub fn unexpected_eof(span: Option<Span>) -> Self {
        Self {
            message: "unexpected end of input".to_string(),
            span,
            kind: ParseErrorKind::UnexpectedEof,
            recoverable: false,
        }
    }

    /// Create an error for unclosed delimiters.
    pub fn unclosed_delimiter(delimiter: &str, span: Option<Span>) -> Self {
        Self {
            message: format!("unclosed {}", delimiter),
            span,
            kind: ParseErrorKind::UnclosedDelimiter,
            recoverable: true,
        }
    }

    /// Create an error for invalid syntax.
    pub fn invalid_syntax(context: &str, span: Option<Span>) -> Self {
        Self {
            message: format!("invalid syntax in {}", context),
            span,
            kind: ParseErrorKind::InvalidSyntax,
            recoverable: true,
        }
    }

    /// Create an error for unknown directives.
    pub fn unknown_directive(directive: &str, span: Option<Span>) -> Self {
        Self {
            message: format!("unknown directive: {}", directive),
            span,
            kind: ParseErrorKind::UnknownDirective,
            recoverable: true,
        }
    }

    /// Set the error kind.
    pub fn with_kind(mut self, kind: ParseErrorKind) -> Self {
        self.kind = kind;
        self
    }

    /// Mark this error as non-recoverable.
    pub fn non_recoverable(mut self) -> Self {
        self.recoverable = false;
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(span) = self.span {
            write!(f, " at bytes {}..{}", span.start, span.end)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// A collection of parse errors encountered during parsing.
#[derive(Debug, Clone, Default)]
pub struct ParseErrors {
    errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Create an empty error collection.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error to the collection.
    pub fn push(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if any errors were collected.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Iterate over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &ParseError> {
        self.errors.iter()
    }

    /// Check if any non-recoverable errors exist.
    pub fn has_fatal(&self) -> bool {
        self.errors.iter().any(|e| !e.recoverable)
    }
}

impl IntoIterator for ParseErrors {
    type Item = ParseError;
    type IntoIter = std::vec::IntoIter<ParseError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}
