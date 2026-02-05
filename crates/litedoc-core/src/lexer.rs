//! Line-based lexer with SIMD-accelerated scanning.
//!
//! The lexer splits input into lines for the block parser.
//! It uses `memchr` for fast newline detection (SIMD on supported platforms).
//!
//! # Performance
//!
//! - Zero-copy: Lines borrow directly from input
//! - SIMD-accelerated newline scanning via `memchr`
//! - Peek/consume API for lookahead without allocations

use crate::span::Span;
use memchr::memchr;

/// A single line from the input with its source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line<'a> {
    /// The line text (without trailing newline).
    pub text: &'a str,
    /// Byte span in the original input.
    pub span: Span,
}

impl<'a> Line<'a> {
    /// Check if this line contains only whitespace.
    #[inline(always)]
    pub fn is_blank(&self) -> bool {
        self.text.bytes().all(|b| b == b' ' || b == b'\t')
    }

    /// Check if the line starts with the given prefix.
    #[inline(always)]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.text.as_bytes().starts_with(prefix.as_bytes())
    }

    /// Get the line text with leading/trailing whitespace removed.
    #[inline(always)]
    pub fn trimmed(&self) -> &str {
        self.text.trim()
    }

    /// Strip a prefix from the line, returning the remainder.
    #[inline(always)]
    pub fn strip_prefix(&self, prefix: &str) -> Option<&'a str> {
        self.text.strip_prefix(prefix)
    }
}

/// Line-based lexer for the block parser.
///
/// Provides peek/consume access to lines with efficient SIMD-accelerated
/// newline scanning.
pub struct Lexer<'a> {
    /// The complete input text.
    input: &'a str,
    /// Input as bytes for efficient scanning.
    bytes: &'a [u8],
    /// Current byte offset.
    offset: usize,
    /// Peeked line (for lookahead).
    peeked: Option<Line<'a>>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            offset: 0,
            peeked: None,
        }
    }

    /// Get the current byte offset.
    #[inline(always)]
    pub fn offset(&self) -> u32 {
        self.offset as u32
    }

    /// Check if all input has been consumed.
    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        self.peeked.is_none() && self.offset >= self.bytes.len()
    }

    /// Peek at the next line without consuming it.
    ///
    /// Returns `None` if at end of input.
    #[inline]
    pub fn peek_line(&mut self) -> Option<&Line<'a>> {
        if self.peeked.is_none() {
            self.peeked = self.read_line();
        }
        self.peeked.as_ref()
    }

    /// Consume and return the next line.
    ///
    /// Returns `None` if at end of input.
    #[inline]
    pub fn next_line(&mut self) -> Option<Line<'a>> {
        if let Some(line) = self.peeked.take() {
            return Some(line);
        }
        self.read_line()
    }

    /// Skip blank lines and return the count skipped.
    #[inline]
    pub fn skip_blank_lines(&mut self) -> usize {
        let mut count = 0;
        while let Some(line) = self.peek_line() {
            if !line.is_blank() {
                break;
            }
            self.next_line();
            count += 1;
        }
        count
    }

    /// Read the next line from input.
    ///
    /// Uses SIMD-accelerated newline scanning via `memchr`.
    #[inline(always)]
    fn read_line(&mut self) -> Option<Line<'a>> {
        if self.offset >= self.bytes.len() {
            return None;
        }

        let start = self.offset;

        // Use memchr for fast newline scanning - this is SIMD accelerated
        let end = match memchr(b'\n', &self.bytes[start..]) {
            Some(pos) => start + pos,
            None => self.bytes.len(),
        };

        // Handle CRLF: check byte before newline is CR
        let text_end = if end > start && self.bytes[end - 1] == b'\r' {
            end - 1
        } else {
            end
        };

        // Advance past newline
        self.offset = if end < self.bytes.len() { end + 1 } else { end };

        Some(Line {
            // SAFETY: Input is valid UTF-8 (guaranteed by &str). We slice at byte positions
            // `start` (previous offset, always valid) and `text_end` (either at newline/CR
            // which are single-byte ASCII, or at input end). Both positions are valid UTF-8
            // char boundaries since newlines and CRs cannot appear mid-character in UTF-8.
            text: unsafe { self.input.get_unchecked(start..text_end) },
            span: Span::new(start as u32, text_end as u32),
        })
    }

    /// Get a slice of the input by span.
    ///
    /// # Safety
    ///
    /// Caller must ensure the span was produced by this lexer instance and
    /// refers to valid byte positions within the input.
    #[inline(always)]
    pub fn slice(&self, span: Span) -> &'a str {
        // SAFETY: Caller guarantees span is valid. Spans from this lexer are always
        // at UTF-8 char boundaries (line starts/ends at newlines which are ASCII).
        unsafe {
            self.input
                .get_unchecked(span.start as usize..span.end as usize)
        }
    }

    /// Get the remaining unconsumed input.
    #[inline(always)]
    pub fn remaining(&self) -> &'a str {
        // SAFETY: self.offset is always valid - initialized to 0 and only advanced
        // by read_line() which sets it to positions after newlines (valid boundaries).
        unsafe { self.input.get_unchecked(self.offset..) }
    }
}
