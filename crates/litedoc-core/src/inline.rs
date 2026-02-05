//! Zero-allocation inline parser for LiteDoc
//!
//! Uses SIMD-accelerated scanning and borrows directly from input.
//! Greedy, left-to-right parsing with no backtracking.

use std::borrow::Cow;

use memchr::{memchr, memchr3};

use crate::ast::{
    AutoLink, CodeSpan, Emphasis, FootnoteRef, Inline, Link, Strikethrough, Strong, Text,
};
use crate::span::Span;

/// Parse inline elements from text content - zero allocation version
#[inline]
pub fn parse_inlines<'a>(text: &'a str, base_offset: u32, _input: &'a str) -> Vec<Inline<'a>> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut parser = InlineParser::new(text, base_offset);
    parser.parse()
}

struct InlineParser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    base_offset: u32,
}

impl<'a> InlineParser<'a> {
    #[inline]
    fn new(text: &'a str, base_offset: u32) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            pos: 0,
            base_offset,
        }
    }

    fn parse(&mut self) -> Vec<Inline<'a>> {
        let mut inlines = Vec::with_capacity(8);
        let mut text_start = 0;

        while self.pos < self.bytes.len() {
            // Fast scan for next special character using SIMD
            let next_special = self.find_next_special();

            if next_special >= self.bytes.len() {
                break;
            }

            self.pos = next_special;
            let c = self.bytes[self.pos];

            let parsed = match c {
                b'\\' => self.try_parse_escape(),
                b'`' => self.try_parse_code_span(&mut inlines, &mut text_start),
                b'[' => self.try_parse_bracket(&mut inlines, &mut text_start),
                b'*' => self.try_parse_asterisk(&mut inlines, &mut text_start),
                b'~' => self.try_parse_tilde(&mut inlines, &mut text_start),
                b'<' => self.try_parse_autolink(&mut inlines, &mut text_start),
                _ => false,
            };

            if !parsed {
                self.pos += 1;
            }
        }

        // Flush remaining text
        if text_start < self.bytes.len() {
            inlines.push(self.make_text_borrowed(text_start, self.bytes.len()));
        }

        inlines
    }

    #[inline(always)]
    fn find_next_special(&self) -> usize {
        let remaining = &self.bytes[self.pos..];

        // Search for both groups of special characters and take the minimum position.
        // memchr3 is SIMD-accelerated so two calls are still fast.
        let common = memchr3(b'*', b'`', b'[', remaining);
        let rare = memchr3(b'\\', b'~', b'<', remaining);

        match (common, rare) {
            (Some(a), Some(b)) => self.pos + a.min(b),
            (Some(a), None) => self.pos + a,
            (None, Some(b)) => self.pos + b,
            (None, None) => self.bytes.len(),
        }
    }

    /// Create text node borrowing directly from input - ZERO ALLOCATION
    #[inline(always)]
    fn make_text_borrowed(&self, start: usize, end: usize) -> Inline<'a> {
        Inline::Text(Text {
            content: Cow::Borrowed(&self.text[start..end]),
            span: Span::new(
                self.base_offset + start as u32,
                self.base_offset + end as u32,
            ),
        })
    }

    #[inline(always)]
    fn flush_text(&self, inlines: &mut Vec<Inline<'a>>, text_start: &mut usize) {
        if *text_start < self.pos {
            inlines.push(self.make_text_borrowed(*text_start, self.pos));
        }
        *text_start = self.pos;
    }

    #[inline]
    fn try_parse_escape(&mut self) -> bool {
        if self.pos + 1 < self.bytes.len() {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    fn try_parse_code_span(
        &mut self,
        inlines: &mut Vec<Inline<'a>>,
        text_start: &mut usize,
    ) -> bool {
        let start = self.pos;

        if let Some(close_offset) = memchr(b'`', &self.bytes[start + 1..]) {
            let close = start + 1 + close_offset;

            self.flush_text(inlines, text_start);

            // Borrow directly from input - ZERO ALLOCATION
            let content = &self.text[start + 1..close];
            inlines.push(Inline::CodeSpan(CodeSpan {
                content: Cow::Borrowed(content),
                span: Span::new(
                    self.base_offset + start as u32,
                    self.base_offset + close as u32 + 1,
                ),
            }));

            self.pos = close + 1;
            *text_start = self.pos;
            true
        } else {
            false
        }
    }

    #[inline]
    fn try_parse_bracket(&mut self, inlines: &mut Vec<Inline<'a>>, text_start: &mut usize) -> bool {
        if self.pos + 1 >= self.bytes.len() {
            return false;
        }

        match self.bytes[self.pos + 1] {
            b'[' => self.try_parse_link(inlines, text_start),
            b'^' => self.try_parse_footnote_ref(inlines, text_start),
            _ => false,
        }
    }

    #[inline]
    fn try_parse_link(&mut self, inlines: &mut Vec<Inline<'a>>, text_start: &mut usize) -> bool {
        let start = self.pos;
        let search_start = start + 2;
        let remaining = &self.bytes[search_start..];

        let mut search_pos = 0;
        while let Some(offset) = memchr(b']', &remaining[search_pos..]) {
            let abs_pos = search_start + search_pos + offset;
            if abs_pos + 1 < self.bytes.len() && self.bytes[abs_pos + 1] == b']' {
                let content = &self.text[search_start..abs_pos];

                self.flush_text(inlines, text_start);

                // Parse label|url - borrow directly
                let (label_text, url) = if let Some(pipe_pos) = content.find('|') {
                    (&content[..pipe_pos], &content[pipe_pos + 1..])
                } else {
                    (content, content)
                };

                let label = vec![Inline::Text(Text {
                    content: Cow::Borrowed(label_text),
                    span: Span::new(
                        self.base_offset + search_start as u32,
                        self.base_offset + (search_start + label_text.len()) as u32,
                    ),
                })];

                inlines.push(Inline::Link(Link {
                    label,
                    url: Cow::Borrowed(url),
                    title: None,
                    span: Span::new(
                        self.base_offset + start as u32,
                        self.base_offset + abs_pos as u32 + 2,
                    ),
                }));

                self.pos = abs_pos + 2;
                *text_start = self.pos;
                return true;
            }
            search_pos += offset + 1;
        }

        false
    }

    #[inline]
    fn try_parse_footnote_ref(
        &mut self,
        inlines: &mut Vec<Inline<'a>>,
        text_start: &mut usize,
    ) -> bool {
        let start = self.pos;
        let label_start = start + 2;

        if let Some(offset) = memchr(b']', &self.bytes[label_start..]) {
            let close = label_start + offset;
            let label = &self.text[label_start..close];

            self.flush_text(inlines, text_start);

            inlines.push(Inline::FootnoteRef(FootnoteRef {
                label: Cow::Borrowed(label),
                span: Span::new(
                    self.base_offset + start as u32,
                    self.base_offset + close as u32 + 1,
                ),
            }));

            self.pos = close + 1;
            *text_start = self.pos;
            true
        } else {
            false
        }
    }

    #[inline]
    fn try_parse_asterisk(
        &mut self,
        inlines: &mut Vec<Inline<'a>>,
        text_start: &mut usize,
    ) -> bool {
        if self.pos + 1 < self.bytes.len() && self.bytes[self.pos + 1] == b'*' {
            self.try_parse_strong(inlines, text_start)
        } else {
            self.try_parse_emphasis(inlines, text_start)
        }
    }

    #[inline]
    fn try_parse_strong(&mut self, inlines: &mut Vec<Inline<'a>>, text_start: &mut usize) -> bool {
        let start = self.pos;
        let content_start = start + 2;

        if content_start >= self.bytes.len() || self.bytes[content_start] == b' ' {
            return false;
        }

        let remaining = &self.bytes[content_start..];
        let mut search_pos = 0;

        while let Some(offset) = memchr(b'*', &remaining[search_pos..]) {
            let abs_pos = content_start + search_pos + offset;

            if abs_pos + 1 < self.bytes.len()
                && self.bytes[abs_pos + 1] == b'*'
                && abs_pos > content_start
                && self.bytes[abs_pos - 1] != b' '
            {
                let content = &self.text[content_start..abs_pos];

                self.flush_text(inlines, text_start);

                // Recursively parse inner content
                let mut inner_parser =
                    InlineParser::new(content, self.base_offset + content_start as u32);
                let inner = inner_parser.parse();

                inlines.push(Inline::Strong(Strong {
                    content: inner,
                    span: Span::new(
                        self.base_offset + start as u32,
                        self.base_offset + abs_pos as u32 + 2,
                    ),
                }));

                self.pos = abs_pos + 2;
                *text_start = self.pos;
                return true;
            }
            search_pos += offset + 1;
        }

        false
    }

    #[inline]
    fn try_parse_emphasis(
        &mut self,
        inlines: &mut Vec<Inline<'a>>,
        text_start: &mut usize,
    ) -> bool {
        let start = self.pos;
        let content_start = start + 1;

        if content_start >= self.bytes.len() || self.bytes[content_start] == b' ' {
            return false;
        }

        let remaining = &self.bytes[content_start..];
        let mut search_pos = 0;

        while let Some(offset) = memchr(b'*', &remaining[search_pos..]) {
            let abs_pos = content_start + search_pos + offset;

            // Skip if it's **
            if abs_pos + 1 < self.bytes.len() && self.bytes[abs_pos + 1] == b'*' {
                search_pos += offset + 2;
                continue;
            }

            if abs_pos > content_start && self.bytes[abs_pos - 1] != b' ' {
                let content = &self.text[content_start..abs_pos];

                self.flush_text(inlines, text_start);

                let mut inner_parser =
                    InlineParser::new(content, self.base_offset + content_start as u32);
                let inner = inner_parser.parse();

                inlines.push(Inline::Emphasis(Emphasis {
                    content: inner,
                    span: Span::new(
                        self.base_offset + start as u32,
                        self.base_offset + abs_pos as u32 + 1,
                    ),
                }));

                self.pos = abs_pos + 1;
                *text_start = self.pos;
                return true;
            }
            search_pos += offset + 1;
        }

        false
    }

    #[inline]
    fn try_parse_tilde(&mut self, inlines: &mut Vec<Inline<'a>>, text_start: &mut usize) -> bool {
        if self.pos + 1 >= self.bytes.len() || self.bytes[self.pos + 1] != b'~' {
            return false;
        }

        let start = self.pos;
        let content_start = start + 2;

        if content_start >= self.bytes.len() || self.bytes[content_start] == b' ' {
            return false;
        }

        let remaining = &self.bytes[content_start..];
        let mut search_pos = 0;

        while let Some(offset) = memchr(b'~', &remaining[search_pos..]) {
            let abs_pos = content_start + search_pos + offset;

            if abs_pos + 1 < self.bytes.len()
                && self.bytes[abs_pos + 1] == b'~'
                && abs_pos > content_start
                && self.bytes[abs_pos - 1] != b' '
            {
                let content = &self.text[content_start..abs_pos];

                self.flush_text(inlines, text_start);

                let mut inner_parser =
                    InlineParser::new(content, self.base_offset + content_start as u32);
                let inner = inner_parser.parse();

                inlines.push(Inline::Strikethrough(Strikethrough {
                    content: inner,
                    span: Span::new(
                        self.base_offset + start as u32,
                        self.base_offset + abs_pos as u32 + 2,
                    ),
                }));

                self.pos = abs_pos + 2;
                *text_start = self.pos;
                return true;
            }
            search_pos += offset + 1;
        }

        false
    }

    #[inline]
    fn try_parse_autolink(
        &mut self,
        inlines: &mut Vec<Inline<'a>>,
        text_start: &mut usize,
    ) -> bool {
        let start = self.pos;

        if let Some(offset) = memchr(b'>', &self.bytes[start + 1..]) {
            let close = start + 1 + offset;
            let url = &self.text[start + 1..close];

            if (url.contains("://") || url.starts_with("mailto:"))
                && !url.contains(' ')
                && !url.contains('\n')
            {
                self.flush_text(inlines, text_start);

                inlines.push(Inline::AutoLink(AutoLink {
                    url: Cow::Borrowed(url),
                    span: Span::new(
                        self.base_offset + start as u32,
                        self.base_offset + close as u32 + 1,
                    ),
                }));

                self.pos = close + 1;
                *text_start = self.pos;
                return true;
            }
        }

        false
    }
}
