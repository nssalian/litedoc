//! Zero-allocation block parser for LiteDoc
//!
//! Borrows directly from input, avoiding String allocations.
//! Features graceful error recovery to continue parsing after errors.

use std::borrow::Cow;

use crate::ast::{
    AttrValue, Block, Callout, CodeBlock, CowStr, Document, Figure, FootnoteDef, Footnotes,
    Heading, HtmlBlock, List, ListItem, ListKind, MathBlock, Metadata, Module, Paragraph, Profile,
    Quote, RawBlock, Table, TableCell, TableRow,
};
use crate::error::{ParseError, ParseErrors};
use crate::lexer::Lexer;
use crate::span::Span;

/// Result type for parsing that includes recovered errors.
#[derive(Debug)]
pub struct ParseResult<'a> {
    /// The parsed document (may be partial if errors occurred).
    pub document: Document<'a>,
    /// Errors encountered during parsing.
    pub errors: ParseErrors,
}

impl<'a> ParseResult<'a> {
    /// Check if parsing completed without errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if any fatal errors occurred.
    pub fn has_fatal_errors(&self) -> bool {
        self.errors.has_fatal()
    }
}

/// LiteDoc parser with configurable profile and error recovery.
pub struct Parser {
    profile: Profile,
    modules: Vec<Module>,
    /// Errors collected during parsing (for recovery mode).
    errors: ParseErrors,
    /// Whether to attempt recovery on errors.
    recover_on_error: bool,
}

impl Parser {
    /// Create a new parser with the given profile.
    #[inline]
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            modules: Vec::new(),
            errors: ParseErrors::new(),
            recover_on_error: true,
        }
    }

    /// Enable or disable error recovery mode.
    ///
    /// When enabled (default), the parser will attempt to continue
    /// parsing after encountering errors, collecting them for later
    /// inspection. When disabled, parsing stops at the first error.
    pub fn with_recovery(mut self, recover: bool) -> Self {
        self.recover_on_error = recover;
        self
    }

    /// Parse with error recovery, returning both document and errors.
    #[inline]
    pub fn parse_with_recovery<'a>(&mut self, input: &'a str) -> ParseResult<'a> {
        self.errors = ParseErrors::new();
        let doc = self.parse_internal(input);
        ParseResult {
            document: doc,
            errors: std::mem::take(&mut self.errors),
        }
    }

    /// Parse the input, returning an error on first failure.
    #[inline]
    pub fn parse<'a>(&mut self, input: &'a str) -> Result<Document<'a>, ParseError> {
        self.errors = ParseErrors::new();
        let doc = self.parse_internal(input);
        if self.errors.is_empty() {
            Ok(doc)
        } else {
            // Return the first error
            Err(self.errors.iter().next().unwrap().clone())
        }
    }

    #[inline]
    fn parse_internal<'a>(&mut self, input: &'a str) -> Document<'a> {
        let mut lexer = Lexer::new(input);

        lexer.skip_blank_lines();

        let profile = self.parse_profile_directive(&mut lexer);
        let modules = self.parse_modules_directive(&mut lexer);
        self.modules = modules.clone();

        lexer.skip_blank_lines();

        let metadata = self.parse_metadata(&mut lexer, input);

        lexer.skip_blank_lines();

        let blocks = self.parse_blocks(&mut lexer, input);

        Document {
            profile: profile.unwrap_or(self.profile),
            modules,
            metadata,
            blocks,
            span: Span::new(0, input.len() as u32),
        }
    }

    /// Record an error during parsing.
    #[inline]
    fn record_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if the given module is enabled.
    #[inline]
    pub fn has_module(&self, module: Module) -> bool {
        self.modules.contains(&module)
    }

    #[inline]
    fn parse_profile_directive(&self, lexer: &mut Lexer) -> Option<Profile> {
        let line = lexer.peek_line()?;
        let trimmed = line.trimmed();

        if let Some(rest) = trimmed.strip_prefix("@profile") {
            let profile = match rest.trim() {
                "litedoc" => Some(Profile::Litedoc),
                "md" => Some(Profile::Md),
                "md-strict" => Some(Profile::MdStrict),
                _ => None,
            };
            if profile.is_some() {
                lexer.next_line();
                lexer.skip_blank_lines();
            }
            profile
        } else {
            None
        }
    }

    #[inline]
    fn parse_modules_directive(&self, lexer: &mut Lexer) -> Vec<Module> {
        let line = lexer.peek_line();
        let trimmed = match line {
            Some(l) => l.trimmed(),
            None => return Vec::new(),
        };

        if let Some(rest) = trimmed.strip_prefix("@modules") {
            let mut modules = Vec::with_capacity(4);
            for part in rest.split(',') {
                match part.trim() {
                    "tables" => modules.push(Module::Tables),
                    "footnotes" => modules.push(Module::Footnotes),
                    "math" => modules.push(Module::Math),
                    "tasks" => modules.push(Module::Tasks),
                    "strikethrough" => modules.push(Module::Strikethrough),
                    "autolink" => modules.push(Module::Autolink),
                    "html" => modules.push(Module::Html),
                    _ => {}
                }
            }
            lexer.next_line();
            lexer.skip_blank_lines();
            modules
        } else {
            Vec::new()
        }
    }

    #[inline]
    fn parse_metadata<'a>(&self, lexer: &mut Lexer, input: &'a str) -> Option<Metadata<'a>> {
        let start_span;
        {
            let line = lexer.peek_line()?;
            let trimmed = line.trimmed();
            if !trimmed.starts_with("---") || !trimmed.contains("meta") {
                return None;
            }
            start_span = line.span;
        }
        lexer.next_line();

        let mut entries: Vec<(CowStr<'a>, AttrValue<'a>)> = Vec::with_capacity(8);
        let mut end_span = start_span;

        loop {
            let (trimmed, span, is_end) = {
                match lexer.peek_line() {
                    Some(line) => {
                        let t = line.trimmed();
                        let s = line.span;
                        let end = t == "---";
                        (t.to_string(), s, end)
                    }
                    None => break,
                }
            };

            if is_end {
                end_span = span;
                lexer.next_line();
                break;
            }

            if let Some(_colon_pos) = trimmed.find(':') {
                // We need to get slices from input, not from trimmed
                let line_start = span.start as usize;
                let line_text = &input[line_start..span.end as usize];

                if let Some(cp) = line_text.find(':') {
                    let key_slice = line_text[..cp].trim();
                    let val_slice = line_text[cp + 1..].trim();

                    let key: CowStr<'a> = Cow::Borrowed(key_slice);
                    let value = self.parse_attr_value(val_slice);
                    entries.push((key, value));
                }
            }

            end_span = span;
            lexer.next_line();
        }

        Some(Metadata {
            entries,
            span: Span::new(start_span.start, end_span.end),
        })
    }

    #[inline]
    fn parse_attr_value<'a>(&self, s: &'a str) -> AttrValue<'a> {
        if s == "true" {
            return AttrValue::Bool(true);
        }
        if s == "false" {
            return AttrValue::Bool(false);
        }

        if s.starts_with('[') && s.ends_with(']') {
            let inner = &s[1..s.len() - 1];
            let items = self.parse_list_items(inner);
            return AttrValue::List(items);
        }

        if let Ok(i) = s.parse::<i64>() {
            return AttrValue::Int(i);
        }

        if s.contains('.') {
            if let Ok(f) = s.parse::<f64>() {
                return AttrValue::Float(f);
            }
        }

        let unquoted = if (s.starts_with('"') && s.ends_with('"'))
            || (s.starts_with('\'') && s.ends_with('\''))
        {
            &s[1..s.len() - 1]
        } else {
            s
        };

        AttrValue::Str(Cow::Borrowed(unquoted))
    }

    #[inline]
    fn parse_list_items<'a>(&self, s: &'a str) -> Vec<AttrValue<'a>> {
        let mut items = Vec::with_capacity(4);
        let mut start = 0;
        let mut in_quotes = false;
        let bytes = s.as_bytes();

        for i in 0..bytes.len() {
            match bytes[i] {
                b'"' | b'\'' => in_quotes = !in_quotes,
                b',' if !in_quotes => {
                    let item = s[start..i].trim();
                    if !item.is_empty() {
                        items.push(self.parse_attr_value(item));
                    }
                    start = i + 1;
                }
                _ => {}
            }
        }

        let item = s[start..].trim();
        if !item.is_empty() {
            items.push(self.parse_attr_value(item));
        }

        items
    }

    #[inline]
    fn parse_blocks<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Vec<Block<'a>> {
        let mut blocks = Vec::with_capacity(16);

        while !lexer.is_eof() {
            lexer.skip_blank_lines();

            if lexer.is_eof() {
                break;
            }

            if let Some(block) = self.parse_block(lexer, input) {
                blocks.push(block);
            }
        }

        blocks
    }

    #[inline]
    fn parse_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (first_byte, trimmed_starts_triple, is_hr, starts_colon, span) = {
            let line = lexer.peek_line()?;
            let trimmed = line.trimmed();
            (
                trimmed.as_bytes().first().copied(),
                trimmed.starts_with("```"),
                trimmed == "---",
                trimmed.starts_with("::"),
                line.span,
            )
        };

        match first_byte {
            Some(b'#') => self.parse_heading(lexer, input),
            Some(b'`') if trimmed_starts_triple => self.parse_code_block(lexer, input),
            Some(b'-') if is_hr => {
                lexer.next_line();
                Some(Block::ThematicBreak(span))
            }
            Some(b':') if starts_colon => self.parse_fenced_block(lexer, input),
            _ => self.parse_paragraph(lexer, input),
        }
    }

    #[inline]
    fn parse_heading<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let line = lexer.next_line()?;
        let text = &input[line.span.start as usize..line.span.end as usize];
        let bytes = text.as_bytes();

        let level = bytes.iter().take_while(|&&b| b == b'#').count() as u8;

        if level == 0 || level > 6 {
            return Some(Block::Paragraph(Paragraph {
                content: crate::inline::parse_inlines(text, line.span.start, input),
                span: line.span,
            }));
        }

        let rest = &text[level as usize..];
        if !rest.starts_with(' ') && !rest.is_empty() {
            return Some(Block::Paragraph(Paragraph {
                content: crate::inline::parse_inlines(text, line.span.start, input),
                span: line.span,
            }));
        }

        let content_text = rest.trim_start();
        let content_offset = line.span.start + (text.len() - content_text.len()) as u32;

        Some(Block::Heading(Heading {
            level,
            content: crate::inline::parse_inlines(content_text, content_offset, input),
            span: line.span,
        }))
    }

    #[inline]
    fn parse_code_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (start_span, lang_start, lang_end) = {
            let open_line = lexer.next_line()?;
            let text = &input[open_line.span.start as usize..open_line.span.end as usize];
            let trimmed = text.trim();
            let after_ticks = trimmed.strip_prefix("```").unwrap_or("");
            let lang = after_ticks.trim();

            // Calculate where lang is in input
            let lang_offset = if lang.is_empty() {
                open_line.span.end as usize
            } else {
                // Find lang in the line
                open_line.span.start as usize + text.find(lang).unwrap_or(0)
            };

            (open_line.span, lang_offset, lang_offset + lang.len())
        };

        let lang = &input[lang_start..lang_end];

        // Content starts after the opening fence line. Add 1 to skip newline if present,
        // but clamp to input length to handle EOF without trailing newline.
        let content_start = (start_span.end as usize + 1).min(input.len());
        let mut content_end = content_start;
        let mut end_span = start_span;

        loop {
            let (is_close, span) = {
                match lexer.peek_line() {
                    Some(line) => (line.trimmed() == "```", line.span),
                    None => break,
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            end_span = span;
            content_end = span.end as usize;
            lexer.next_line();
        }

        let content = if content_start < content_end && content_end <= input.len() {
            &input[content_start..content_end]
        } else {
            ""
        };

        Some(Block::CodeBlock(CodeBlock {
            lang: Cow::Borrowed(lang),
            content: Cow::Borrowed(content),
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_fenced_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (block_type, span) = {
            let line = lexer.peek_line()?;
            let trimmed = line.trimmed();
            let after_colons = trimmed.strip_prefix("::")?;
            let bt = after_colons
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            (bt, line.span)
        };

        match block_type.as_str() {
            "list" => self.parse_list_block(lexer, input),
            "callout" => self.parse_callout_block(lexer, input),
            "quote" => self.parse_quote_block(lexer, input),
            "figure" => self.parse_figure_block(lexer, input),
            "table" => self.parse_table_block(lexer, input),
            "footnotes" => self.parse_footnotes_block(lexer, input),
            "math" => self.parse_math_block(lexer, input),
            "html" => self.parse_html_block(lexer, input),
            _ => {
                // Record error for unknown directive but continue with raw block
                if self.recover_on_error && !block_type.is_empty() {
                    self.record_error(ParseError::unknown_directive(&block_type, Some(span)));
                }
                self.parse_raw_fenced_block(lexer, input)
            }
        }
    }

    #[inline]
    fn parse_html_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        // Check if HTML module is enabled
        if !self.has_module(Module::Html) {
            return self.parse_raw_fenced_block(lexer, input);
        }

        let start_span = lexer.next_line()?.span;

        // Content starts after opening fence, clamped to input length
        let content_start = (start_span.end as usize + 1).min(input.len());
        let mut content_end = content_start;
        let mut end_span = start_span;

        loop {
            let (is_close, span) = {
                match lexer.peek_line() {
                    Some(line) => (line.trimmed() == "::", line.span),
                    None => {
                        // Record unclosed HTML block error
                        self.record_error(ParseError::unclosed_delimiter(
                            "HTML block",
                            Some(start_span),
                        ));
                        break;
                    }
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            content_end = span.end as usize;
            end_span = span;
            lexer.next_line();
        }

        let content = if content_start < content_end && content_end <= input.len() {
            &input[content_start..content_end]
        } else {
            ""
        };

        Some(Block::Html(HtmlBlock {
            content: Cow::Borrowed(content),
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_list_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (start_span, kind, start_num) = {
            let open_line = lexer.next_line()?;
            let text = &input[open_line.span.start as usize..open_line.span.end as usize];
            let trimmed = text.trim();
            let after_list = trimmed.strip_prefix("::list").unwrap_or("").trim();

            let mut k = ListKind::Unordered;
            let mut sn: Option<u64> = None;

            for part in after_list.split_whitespace() {
                match part {
                    "ordered" => k = ListKind::Ordered,
                    "unordered" => k = ListKind::Unordered,
                    _ if part.starts_with("start=") => {
                        sn = part[6..].parse().ok();
                    }
                    _ => {}
                }
            }
            (open_line.span, k, sn)
        };

        let mut items: Vec<ListItem<'a>> = Vec::with_capacity(8);
        let mut item_start: Option<u32> = None;
        let mut item_end: u32 = start_span.end;
        let mut end_span = start_span;
        let mut last_span = start_span;

        let finalize_item = |items: &mut Vec<ListItem<'a>>, start: Option<u32>, end: u32| {
            if let Some(start) = start {
                let content_slice = &input[start as usize..end as usize];
                let content = crate::inline::parse_inlines(content_slice, start, input);
                items.push(ListItem {
                    blocks: vec![Block::Paragraph(Paragraph {
                        content,
                        span: Span::new(start, end),
                    })],
                    span: Span::new(start, end),
                });
            }
        };

        while let Some(&line) = lexer.peek_line() {
            let text = &input[line.span.start as usize..line.span.end as usize];
            let trimmed = text.trim();

            if trimmed == "::" {
                lexer.next_line();
                finalize_item(&mut items, item_start.take(), item_end);
                end_span = line.span;
                break;
            }

            if trimmed.starts_with("- ") {
                lexer.next_line();
                finalize_item(&mut items, item_start.take(), item_end);
                let dash_offset = text.find("- ").unwrap_or(0);
                item_start = Some(line.span.start + dash_offset as u32 + 2);
                item_end = line.span.end;
                last_span = line.span;
                end_span = line.span;
                continue;
            }

            if trimmed.starts_with("| ") {
                lexer.next_line();
                item_end = line.span.end;
                last_span = line.span;
                end_span = line.span;
                continue;
            }

            if line.is_blank() {
                lexer.next_line();
                last_span = line.span;
                end_span = line.span;
                continue;
            }

            if trimmed.starts_with("::")
                || trimmed.starts_with("```")
                || trimmed.starts_with('#')
                || trimmed.starts_with("@profile")
                || trimmed.starts_with("@modules")
                || trimmed == "---"
                || trimmed.starts_with("--- meta ---")
            {
                self.record_error(ParseError::unclosed_delimiter("::list", Some(start_span)));
                finalize_item(&mut items, item_start.take(), item_end);
                end_span = last_span;
                break;
            }

            self.record_error(ParseError::invalid_syntax("list item", Some(line.span)));
            finalize_item(&mut items, item_start.take(), item_end);
            end_span = last_span;
            break;
        }

        Some(Block::List(List {
            kind,
            start: start_num,
            items,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_callout_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (start_span, kind, title) = {
            let open_line = lexer.next_line()?;
            let text = &input[open_line.span.start as usize..open_line.span.end as usize];
            let trimmed = text.trim();
            let after_callout = trimmed.strip_prefix("::callout").unwrap_or("").trim();
            let (k, t) = self.parse_callout_attrs(after_callout, input, open_line.span.start);
            (open_line.span, k, t)
        };

        let (blocks, end_span) = self.parse_until_fence_close(lexer, input);

        Some(Block::Callout(Callout {
            kind,
            title,
            blocks,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_callout_attrs<'a>(
        &self,
        s: &str,
        _input: &'a str,
        _base: u32,
    ) -> (CowStr<'a>, Option<CowStr<'a>>) {
        let mut kind: CowStr<'a> = Cow::Owned("note".to_string());
        let mut title: Option<CowStr<'a>> = None;

        let mut remaining = s;
        while !remaining.is_empty() {
            let eq_pos = match remaining.find('=') {
                Some(p) => p,
                None => break,
            };

            let key = remaining[..eq_pos].trim();
            remaining = remaining[eq_pos + 1..].trim_start();

            if remaining.starts_with('"') {
                if let Some(end) = remaining[1..].find('"') {
                    let val = &remaining[1..end + 1];
                    remaining = remaining[end + 2..].trim_start();

                    match key {
                        "type" => kind = Cow::Owned(val.to_string()),
                        "title" => title = Some(Cow::Owned(val.to_string())),
                        _ => {}
                    }
                } else {
                    break;
                }
            } else {
                let end = remaining.find(' ').unwrap_or(remaining.len());
                let val = &remaining[..end];
                remaining = remaining[end..].trim_start();

                match key {
                    "type" => kind = Cow::Owned(val.to_string()),
                    "title" => title = Some(Cow::Owned(val.to_string())),
                    _ => {}
                }
            }
        }

        (kind, title)
    }

    #[inline]
    fn parse_quote_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let start_span = lexer.next_line()?.span;
        let (blocks, end_span) = self.parse_until_fence_close(lexer, input);

        Some(Block::Quote(Quote {
            blocks,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_figure_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (start_span, src, alt, caption) = {
            let open_line = lexer.next_line()?;
            let text = &input[open_line.span.start as usize..open_line.span.end as usize];
            let trimmed = text.trim();
            let after_figure = trimmed.strip_prefix("::figure").unwrap_or("").trim();
            let (s, a, c) = self.parse_figure_attrs(after_figure);
            (open_line.span, s, a, c)
        };

        let mut end_span = start_span;
        if let Some(line) = lexer.peek_line() {
            if line.trimmed() == "::" {
                end_span = line.span;
                lexer.next_line();
            }
        }

        Some(Block::Figure(Figure {
            src,
            alt,
            caption,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_figure_attrs<'a>(&self, s: &str) -> (CowStr<'a>, CowStr<'a>, Option<CowStr<'a>>) {
        let mut src: CowStr<'a> = Cow::Owned(String::new());
        let mut alt: CowStr<'a> = Cow::Owned(String::new());
        let mut caption: Option<CowStr<'a>> = None;

        let mut remaining = s;
        while !remaining.is_empty() {
            let eq_pos = match remaining.find('=') {
                Some(p) => p,
                None => break,
            };

            let key = remaining[..eq_pos].trim();
            remaining = remaining[eq_pos + 1..].trim_start();

            if remaining.starts_with('"') {
                if let Some(end) = remaining[1..].find('"') {
                    let val = remaining[1..end + 1].to_string();
                    remaining = remaining[end + 2..].trim_start();

                    match key {
                        "src" => src = Cow::Owned(val),
                        "alt" => alt = Cow::Owned(val),
                        "caption" => caption = Some(Cow::Owned(val)),
                        _ => {}
                    }
                } else {
                    break;
                }
            } else {
                let end = remaining.find(' ').unwrap_or(remaining.len());
                let val = remaining[..end].to_string();
                remaining = remaining[end..].trim_start();

                match key {
                    "src" => src = Cow::Owned(val),
                    "alt" => alt = Cow::Owned(val),
                    "caption" => caption = Some(Cow::Owned(val)),
                    _ => {}
                }
            }
        }

        (src, alt, caption)
    }

    #[inline]
    fn parse_table_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let start_span = lexer.next_line()?.span;

        let mut rows: Vec<TableRow<'a>> = Vec::with_capacity(8);
        let mut end_span = start_span;
        let mut found_separator = false;

        loop {
            let (is_close, is_sep, is_row, span, line_text) = {
                match lexer.peek_line() {
                    Some(line) => {
                        let text = &input[line.span.start as usize..line.span.end as usize];
                        let trimmed = text.trim();
                        (
                            trimmed == "::",
                            trimmed.starts_with('|') && trimmed.contains("---"),
                            trimmed.starts_with('|'),
                            line.span,
                            trimmed,
                        )
                    }
                    None => break,
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            if is_sep {
                found_separator = true;
                end_span = span;
                lexer.next_line();
                continue;
            }

            if is_row {
                let cells = self.parse_table_row(line_text, span.start, input);
                let is_header = !found_separator && rows.is_empty();
                rows.push(TableRow {
                    cells,
                    header: is_header,
                    span,
                });
                end_span = span;
                lexer.next_line();
                continue;
            }

            self.record_error(ParseError::unclosed_delimiter("::table", Some(start_span)));
            break;
        }

        Some(Block::Table(Table {
            rows,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_table_row<'a>(
        &self,
        line: &'a str,
        base_offset: u32,
        input: &'a str,
    ) -> Vec<TableCell<'a>> {
        let mut cells = Vec::with_capacity(8);
        let mut offset = base_offset;

        for (i, part) in line.split('|').enumerate() {
            if i == 0 {
                offset += part.len() as u32 + 1;
                continue;
            }

            let trimmed = part.trim();
            if trimmed.is_empty() {
                offset += part.len() as u32 + 1;
                continue;
            }

            let content = crate::inline::parse_inlines(trimmed, offset, input);
            cells.push(TableCell {
                content,
                span: Span::new(offset, offset + part.len() as u32),
            });

            offset += part.len() as u32 + 1;
        }

        cells
    }

    #[inline]
    fn parse_footnotes_block<'a>(
        &mut self,
        lexer: &mut Lexer,
        input: &'a str,
    ) -> Option<Block<'a>> {
        let start_span = lexer.next_line()?.span;

        let mut defs: Vec<FootnoteDef<'a>> = Vec::with_capacity(4);
        let mut end_span = start_span;

        loop {
            let (is_close, is_def, span, label, content_text) = {
                match lexer.peek_line() {
                    Some(line) => {
                        let text = &input[line.span.start as usize..line.span.end as usize];
                        let trimmed = text.trim();
                        let is_close = trimmed == "::";
                        let mut is_def = false;
                        let mut label = "";
                        let mut content = "";

                        if trimmed.starts_with("[^") {
                            if let Some(bracket_end) = trimmed.find("]:") {
                                is_def = true;
                                label = &trimmed[2..bracket_end];
                                content = trimmed[bracket_end + 2..].trim();
                            }
                        }
                        (is_close, is_def, line.span, label, content)
                    }
                    None => break,
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            if is_def {
                let content_inlines = crate::inline::parse_inlines(content_text, span.start, input);
                defs.push(FootnoteDef {
                    label: Cow::Owned(label.to_string()),
                    blocks: vec![Block::Paragraph(Paragraph {
                        content: content_inlines,
                        span,
                    })],
                    span,
                });
            }

            end_span = span;
            lexer.next_line();
        }

        Some(Block::Footnotes(Footnotes {
            defs,
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_math_block<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let (start_span, display) = {
            let open_line = lexer.next_line()?;
            let text = &input[open_line.span.start as usize..open_line.span.end as usize];
            let trimmed = text.trim();
            let d = trimmed.contains("block") || trimmed.contains("display");
            (open_line.span, d)
        };

        // Content starts after opening fence, clamped to input length
        let content_start = (start_span.end as usize + 1).min(input.len());
        let mut content_end = content_start;
        let mut end_span = start_span;

        loop {
            let (is_close, span) = {
                match lexer.peek_line() {
                    Some(line) => (line.trimmed() == "::", line.span),
                    None => break,
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            content_end = span.end as usize;
            end_span = span;
            lexer.next_line();
        }

        let content = if content_start < content_end && content_end <= input.len() {
            &input[content_start..content_end]
        } else {
            ""
        };

        Some(Block::Math(MathBlock {
            display,
            content: Cow::Borrowed(content),
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_raw_fenced_block<'a>(
        &mut self,
        lexer: &mut Lexer,
        input: &'a str,
    ) -> Option<Block<'a>> {
        let start_span = lexer.next_line()?.span;

        // Content starts after opening fence, clamped to input length
        let content_start = (start_span.end as usize + 1).min(input.len());
        let mut content_end = content_start;
        let mut end_span = start_span;

        loop {
            let (is_close, span) = {
                match lexer.peek_line() {
                    Some(line) => (line.trimmed() == "::", line.span),
                    None => break,
                }
            };

            if is_close {
                end_span = span;
                lexer.next_line();
                break;
            }

            content_end = span.end as usize;
            end_span = span;
            lexer.next_line();
        }

        let content = if content_start < content_end && content_end <= input.len() {
            &input[content_start..content_end]
        } else {
            ""
        };

        Some(Block::Raw(RawBlock {
            content: Cow::Borrowed(content),
            span: Span::new(start_span.start, end_span.end),
        }))
    }

    #[inline]
    fn parse_until_fence_close<'a>(
        &mut self,
        lexer: &mut Lexer,
        input: &'a str,
    ) -> (Vec<Block<'a>>, Span) {
        let mut blocks = Vec::with_capacity(4);
        let mut para_start: Option<u32> = None;
        let mut para_end: u32 = 0;
        let mut end_span = Span::new(0, 0);

        loop {
            let (is_close, is_blank, span) = {
                match lexer.next_line() {
                    Some(line) => (line.trimmed() == "::", line.is_blank(), line.span),
                    None => break,
                }
            };

            if is_close {
                if let Some(start) = para_start {
                    let content_slice = &input[start as usize..para_end as usize];
                    let content = crate::inline::parse_inlines(content_slice, start, input);
                    blocks.push(Block::Paragraph(Paragraph {
                        content,
                        span: Span::new(start, para_end),
                    }));
                }
                end_span = span;
                break;
            }

            if is_blank {
                if let Some(start) = para_start.take() {
                    let content_slice = &input[start as usize..para_end as usize];
                    let content = crate::inline::parse_inlines(content_slice, start, input);
                    blocks.push(Block::Paragraph(Paragraph {
                        content,
                        span: Span::new(start, para_end),
                    }));
                }
            } else {
                if para_start.is_none() {
                    para_start = Some(span.start);
                }
                para_end = span.end;
            }

            end_span = span;
        }

        (blocks, end_span)
    }

    #[inline]
    fn parse_paragraph<'a>(&mut self, lexer: &mut Lexer, input: &'a str) -> Option<Block<'a>> {
        let mut start_span: Option<Span> = None;
        let mut end_span = Span::new(0, 0);

        loop {
            let should_break = {
                match lexer.peek_line() {
                    Some(line) => {
                        if line.is_blank() {
                            true
                        } else {
                            let trimmed = line.trimmed();
                            let first = trimmed.as_bytes().first().copied();
                            match first {
                                Some(b'#') | Some(b':') => true,
                                Some(b'`') if trimmed.starts_with("```") => true,
                                Some(b'-') if trimmed == "---" => true,
                                _ => false,
                            }
                        }
                    }
                    None => true,
                }
            };

            if should_break {
                break;
            }

            let line = lexer.next_line().unwrap();
            if start_span.is_none() {
                start_span = Some(line.span);
            }
            end_span = line.span;
        }

        let start = start_span?;
        let content_slice = &input[start.start as usize..end_span.end as usize];
        let content = crate::inline::parse_inlines(content_slice, start.start, input);

        Some(Block::Paragraph(Paragraph {
            content,
            span: Span::new(start.start, end_span.end),
        }))
    }
}
