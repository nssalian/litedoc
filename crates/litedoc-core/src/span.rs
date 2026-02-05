//! Source location tracking for AST nodes.
//!
//! Every AST node includes a `Span` indicating its position in the source text.
//! This enables precise error reporting and source mapping.

/// A byte range in the source text.
///
/// Spans use byte offsets (not character offsets) for efficiency.
/// Both `start` and `end` are inclusive-exclusive: `[start, end)`.
///
/// # Example
///
/// ```rust
/// use litedoc_core::span::Span;
///
/// let span = Span::new(0, 10);
/// assert_eq!(span.len(), 10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Starting byte offset (inclusive).
    pub start: u32,
    /// Ending byte offset (exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new span from byte offsets.
    #[inline]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Get the length of this span in bytes.
    #[inline]
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Check if this span is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Check if this span contains a byte offset.
    #[inline]
    pub const fn contains(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Merge two spans into one covering both.
    #[inline]
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
