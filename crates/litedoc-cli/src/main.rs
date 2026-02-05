//! LiteDoc CLI - Parse, validate, and convert LiteDoc documents
//!
//! Usage:
//!   ldcli [OPTIONS] <FILE>
//!
//! Commands:
//!   parse     Parse and display document structure (default)
//!   validate  Check document for errors
//!   stats     Show document statistics

use std::env;
use std::fs;
use std::process;

use litedoc_core::{ast, Block, Document, Inline, Parser, Profile};
use serde::Serialize;

fn main() {
    let args: Vec<String> = env::args().collect();

    match run(&args) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let config = parse_args(args)?;

    let input = fs::read_to_string(&config.file)
        .map_err(|e| format!("failed to read '{}': {}", config.file, e))?;

    // Infer profile from extension
    let profile = if config.file.ends_with(".md") {
        Profile::Md
    } else {
        Profile::Litedoc
    };

    let mut parser = Parser::new(profile);

    match config.command {
        Command::Parse => cmd_parse(&mut parser, &input, &config),
        Command::Validate => cmd_validate(&mut parser, &input, &config),
        Command::Stats => cmd_stats(&mut parser, &input),
    }
}

#[derive(Debug)]
struct Config {
    command: Command,
    file: String,
    format: OutputFormat,
    verbose: bool,
}

#[derive(Debug, Clone, Copy)]
enum Command {
    Parse,
    Validate,
    Stats,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut command = Command::Parse;
    let mut format = OutputFormat::Text;
    let mut verbose = false;
    let mut file = None;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("ldcli {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "-v" | "--verbose" => verbose = true,
            "-j" | "--json" => format = OutputFormat::Json,
            "parse" => command = Command::Parse,
            "validate" => command = Command::Validate,
            "stats" => command = Command::Stats,
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option: {}", arg));
            }
            _ => {
                if file.is_some() {
                    return Err("multiple files specified".to_string());
                }
                file = Some(arg.clone());
            }
        }
        i += 1;
    }

    let file = file.ok_or_else(|| "no input file specified".to_string())?;

    Ok(Config {
        command,
        file,
        format,
        verbose,
    })
}

fn print_help() {
    eprintln!(
        r#"ldcli - LiteDoc document parser and validator

USAGE:
    ldcli [OPTIONS] [COMMAND] <FILE>

COMMANDS:
    parse       Parse and display document structure (default)
    validate    Check document for errors without output
    stats       Show document statistics

OPTIONS:
    -v, --verbose    Show detailed AST structure
    -j, --json       Output in JSON format
    -h, --help       Print help information
    -V, --version    Print version information

EXAMPLES:
    ldcli document.ld           Parse a LiteDoc file
    ldcli -v document.ld        Parse with verbose output
    ldcli -j document.ld        Output AST as JSON
    ldcli validate document.ld  Validate without output
    ldcli stats document.ld     Show document statistics
"#
    );
}

// =============================================================================
// Parse Command
// =============================================================================

fn cmd_parse(parser: &mut Parser, input: &str, config: &Config) -> Result<(), String> {
    let result = parser.parse_with_recovery(input);

    // Report any errors
    for error in result.errors.iter() {
        eprintln!("warning: {}", error);
    }

    match config.format {
        OutputFormat::Json => print_json(&result.document),
        OutputFormat::Text => {
            if config.verbose {
                print_document_verbose(&result.document);
            } else {
                print_document_summary(&result.document);
            }
        }
    }

    Ok(())
}

// =============================================================================
// Validate Command
// =============================================================================

fn cmd_validate(parser: &mut Parser, input: &str, config: &Config) -> Result<(), String> {
    let result = parser.parse_with_recovery(input);

    if result.errors.is_empty() {
        if !matches!(config.format, OutputFormat::Json) {
            println!("Valid: no errors found");
        } else {
            println!(r#"{{"valid": true, "errors": []}}"#);
        }
        Ok(())
    } else {
        if matches!(config.format, OutputFormat::Json) {
            let errors: Vec<_> = result
                .errors
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "message": e.message,
                        "span": e.span.map(|s| serde_json::json!({"start": s.start, "end": s.end})),
                        "recoverable": e.recoverable
                    })
                })
                .collect();
            println!("{}", serde_json::json!({"valid": false, "errors": errors}));
        } else {
            eprintln!("Invalid: {} error(s) found", result.errors.len());
            for error in result.errors.iter() {
                eprintln!("  - {}", error);
            }
        }
        Err(format!("{} error(s) found", result.errors.len()))
    }
}

// =============================================================================
// Stats Command
// =============================================================================

fn cmd_stats(parser: &mut Parser, input: &str) -> Result<(), String> {
    let result = parser.parse_with_recovery(input);
    let doc = &result.document;

    let stats = DocumentStats::from_document(doc, input);

    println!("Document Statistics");
    println!("-------------------");
    println!("Profile:      {:?}", doc.profile);
    println!("Modules:      {}", doc.modules.len());
    println!("Has metadata: {}", doc.metadata.is_some());
    println!();
    println!("Content:");
    println!("  Total blocks:   {}", stats.total_blocks);
    println!("  Headings:       {}", stats.headings);
    println!("  Paragraphs:     {}", stats.paragraphs);
    println!("  Code blocks:    {}", stats.code_blocks);
    println!("  Lists:          {}", stats.lists);
    println!("  Tables:         {}", stats.tables);
    println!("  Callouts:       {}", stats.callouts);
    println!();
    println!("Size:");
    println!("  Characters:     {}", stats.chars);
    println!("  Words (est.):   {}", stats.words);
    println!("  Lines:          {}", stats.lines);
    println!();
    println!("Errors:         {}", result.errors.len());

    Ok(())
}

struct DocumentStats {
    total_blocks: usize,
    headings: usize,
    paragraphs: usize,
    code_blocks: usize,
    lists: usize,
    tables: usize,
    callouts: usize,
    chars: usize,
    words: usize,
    lines: usize,
}

impl DocumentStats {
    fn from_document(doc: &Document, input: &str) -> Self {
        let mut stats = Self {
            total_blocks: 0,
            headings: 0,
            paragraphs: 0,
            code_blocks: 0,
            lists: 0,
            tables: 0,
            callouts: 0,
            chars: input.len(),
            words: input.split_whitespace().count(),
            lines: input.lines().count(),
        };

        stats.count_blocks(&doc.blocks);
        stats
    }

    fn count_blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            self.total_blocks += 1;
            match block {
                Block::Heading(_) => self.headings += 1,
                Block::Paragraph(_) => self.paragraphs += 1,
                Block::CodeBlock(_) => self.code_blocks += 1,
                Block::List(l) => {
                    self.lists += 1;
                    for item in &l.items {
                        self.count_blocks(&item.blocks);
                    }
                }
                Block::Table(_) => self.tables += 1,
                Block::Callout(c) => {
                    self.callouts += 1;
                    self.count_blocks(&c.blocks);
                }
                Block::Quote(q) => self.count_blocks(&q.blocks),
                _ => {}
            }
        }
    }
}

// =============================================================================
// JSON Output
// =============================================================================

#[derive(Serialize)]
struct JsonDocument<'a> {
    profile: &'a str,
    modules: Vec<&'a str>,
    metadata: Option<JsonMetadata<'a>>,
    blocks: Vec<JsonBlock<'a>>,
}

#[derive(Serialize)]
struct JsonMetadata<'a> {
    entries: Vec<(&'a str, serde_json::Value)>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum JsonBlock<'a> {
    Heading {
        level: u8,
        content: Vec<JsonInline<'a>>,
    },
    Paragraph {
        content: Vec<JsonInline<'a>>,
    },
    CodeBlock {
        lang: &'a str,
        content: &'a str,
    },
    List {
        kind: &'a str,
        items: Vec<Vec<JsonBlock<'a>>>,
    },
    Callout {
        kind: &'a str,
        title: Option<&'a str>,
        blocks: Vec<JsonBlock<'a>>,
    },
    Quote {
        blocks: Vec<JsonBlock<'a>>,
    },
    Table {
        rows: Vec<JsonTableRow<'a>>,
    },
    Figure {
        src: &'a str,
        alt: &'a str,
        caption: Option<&'a str>,
    },
    Math {
        display: bool,
        content: &'a str,
    },
    ThematicBreak,
    Html {
        content: &'a str,
    },
    Raw {
        content: &'a str,
    },
    Footnotes {
        defs: Vec<JsonFootnoteDef<'a>>,
    },
}

#[derive(Serialize)]
struct JsonTableRow<'a> {
    header: bool,
    cells: Vec<Vec<JsonInline<'a>>>,
}

#[derive(Serialize)]
struct JsonFootnoteDef<'a> {
    label: &'a str,
    blocks: Vec<JsonBlock<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum JsonInline<'a> {
    Text {
        content: &'a str,
    },
    Emphasis {
        content: Vec<JsonInline<'a>>,
    },
    Strong {
        content: Vec<JsonInline<'a>>,
    },
    CodeSpan {
        content: &'a str,
    },
    Link {
        label: Vec<JsonInline<'a>>,
        url: &'a str,
    },
    AutoLink {
        url: &'a str,
    },
    Strikethrough {
        content: Vec<JsonInline<'a>>,
    },
    FootnoteRef {
        label: &'a str,
    },
    HardBreak,
    SoftBreak,
}

fn print_json(doc: &Document) {
    let json_doc = convert_document(doc);
    println!("{}", serde_json::to_string_pretty(&json_doc).unwrap());
}

fn convert_document<'a>(doc: &'a Document) -> JsonDocument<'a> {
    JsonDocument {
        profile: match doc.profile {
            Profile::Litedoc => "litedoc",
            Profile::Md => "md",
            Profile::MdStrict => "md-strict",
        },
        modules: doc
            .modules
            .iter()
            .map(|m| match m {
                ast::Module::Tables => "tables",
                ast::Module::Footnotes => "footnotes",
                ast::Module::Math => "math",
                ast::Module::Tasks => "tasks",
                ast::Module::Strikethrough => "strikethrough",
                ast::Module::Autolink => "autolink",
                ast::Module::Html => "html",
            })
            .collect(),
        metadata: doc.metadata.as_ref().map(|m| JsonMetadata {
            entries: m
                .entries
                .iter()
                .map(|(k, v)| (k.as_ref(), convert_attr_value(v)))
                .collect(),
        }),
        blocks: doc.blocks.iter().map(convert_block).collect(),
    }
}

fn convert_attr_value(value: &ast::AttrValue) -> serde_json::Value {
    match value {
        ast::AttrValue::Str(s) => serde_json::Value::String(s.to_string()),
        ast::AttrValue::Bool(b) => serde_json::Value::Bool(*b),
        ast::AttrValue::Int(i) => serde_json::Value::Number((*i).into()),
        ast::AttrValue::Float(f) => serde_json::json!(*f),
        ast::AttrValue::List(items) => {
            serde_json::Value::Array(items.iter().map(convert_attr_value).collect())
        }
    }
}

fn convert_block<'a>(block: &'a Block) -> JsonBlock<'a> {
    match block {
        Block::Heading(h) => JsonBlock::Heading {
            level: h.level,
            content: h.content.iter().map(convert_inline).collect(),
        },
        Block::Paragraph(p) => JsonBlock::Paragraph {
            content: p.content.iter().map(convert_inline).collect(),
        },
        Block::CodeBlock(c) => JsonBlock::CodeBlock {
            lang: &c.lang,
            content: &c.content,
        },
        Block::List(l) => JsonBlock::List {
            kind: match l.kind {
                ast::ListKind::Ordered => "ordered",
                ast::ListKind::Unordered => "unordered",
            },
            items: l
                .items
                .iter()
                .map(|item| item.blocks.iter().map(convert_block).collect())
                .collect(),
        },
        Block::Callout(c) => JsonBlock::Callout {
            kind: &c.kind,
            title: c.title.as_deref(),
            blocks: c.blocks.iter().map(convert_block).collect(),
        },
        Block::Quote(q) => JsonBlock::Quote {
            blocks: q.blocks.iter().map(convert_block).collect(),
        },
        Block::Table(t) => JsonBlock::Table {
            rows: t
                .rows
                .iter()
                .map(|row| JsonTableRow {
                    header: row.header,
                    cells: row
                        .cells
                        .iter()
                        .map(|cell| cell.content.iter().map(convert_inline).collect())
                        .collect(),
                })
                .collect(),
        },
        Block::Figure(f) => JsonBlock::Figure {
            src: &f.src,
            alt: &f.alt,
            caption: f.caption.as_deref(),
        },
        Block::Math(m) => JsonBlock::Math {
            display: m.display,
            content: &m.content,
        },
        Block::ThematicBreak(_) => JsonBlock::ThematicBreak,
        Block::Html(h) => JsonBlock::Html {
            content: &h.content,
        },
        Block::Raw(r) => JsonBlock::Raw {
            content: &r.content,
        },
        Block::Footnotes(f) => JsonBlock::Footnotes {
            defs: f
                .defs
                .iter()
                .map(|def| JsonFootnoteDef {
                    label: &def.label,
                    blocks: def.blocks.iter().map(convert_block).collect(),
                })
                .collect(),
        },
    }
}

fn convert_inline<'a>(inline: &'a Inline) -> JsonInline<'a> {
    match inline {
        Inline::Text(t) => JsonInline::Text {
            content: &t.content,
        },
        Inline::Emphasis(e) => JsonInline::Emphasis {
            content: e.content.iter().map(convert_inline).collect(),
        },
        Inline::Strong(s) => JsonInline::Strong {
            content: s.content.iter().map(convert_inline).collect(),
        },
        Inline::CodeSpan(c) => JsonInline::CodeSpan {
            content: &c.content,
        },
        Inline::Link(l) => JsonInline::Link {
            label: l.label.iter().map(convert_inline).collect(),
            url: &l.url,
        },
        Inline::AutoLink(a) => JsonInline::AutoLink { url: &a.url },
        Inline::Strikethrough(s) => JsonInline::Strikethrough {
            content: s.content.iter().map(convert_inline).collect(),
        },
        Inline::FootnoteRef(f) => JsonInline::FootnoteRef { label: &f.label },
        Inline::HardBreak(_) => JsonInline::HardBreak,
        Inline::SoftBreak(_) => JsonInline::SoftBreak,
    }
}

// =============================================================================
// Text Output
// =============================================================================

fn print_document_summary(doc: &Document) {
    println!("Profile: {:?}", doc.profile);

    if !doc.modules.is_empty() {
        print!("Modules: ");
        for (i, m) in doc.modules.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{:?}", m);
        }
        println!();
    }

    if let Some(ref meta) = doc.metadata {
        println!("Metadata: {} entries", meta.entries.len());
        for (key, value) in &meta.entries {
            println!("  {}: {}", key, format_attr_value(value));
        }
    }

    println!("Blocks: {}", doc.blocks.len());
    for (i, block) in doc.blocks.iter().enumerate() {
        println!("  [{}] {}", i + 1, describe_block(block));
    }
}

fn print_document_verbose(doc: &Document) {
    println!("=== LiteDoc AST ===");
    println!();
    println!("Profile: {:?}", doc.profile);
    println!("Modules: {:?}", doc.modules);
    println!("Span: {}..{}", doc.span.start, doc.span.end);
    println!();

    if let Some(ref meta) = doc.metadata {
        println!("--- Metadata ---");
        for (key, value) in &meta.entries {
            println!("  {}: {}", key, format_attr_value(value));
        }
        println!();
    }

    println!("--- Blocks ---");
    for (i, block) in doc.blocks.iter().enumerate() {
        println!();
        println!("[{}] {}", i + 1, describe_block(block));
        print_block_verbose(block, 1);
    }
}

fn describe_block(block: &Block) -> String {
    match block {
        Block::Heading(h) => format!("Heading (level {})", h.level),
        Block::Paragraph(_) => "Paragraph".to_string(),
        Block::List(l) => format!("List ({:?}, {} items)", l.kind, l.items.len()),
        Block::CodeBlock(c) => format!("CodeBlock (lang: {})", c.lang),
        Block::Callout(c) => format!("Callout (type: {}, title: {:?})", c.kind, c.title),
        Block::Quote(_) => "Quote".to_string(),
        Block::Figure(f) => format!("Figure (src: {})", f.src),
        Block::Table(t) => format!("Table ({} rows)", t.rows.len()),
        Block::Footnotes(f) => format!("Footnotes ({} defs)", f.defs.len()),
        Block::Math(m) => format!("Math (display: {})", m.display),
        Block::ThematicBreak(_) => "ThematicBreak".to_string(),
        Block::Html(_) => "Html".to_string(),
        Block::Raw(_) => "Raw".to_string(),
    }
}

fn print_block_verbose(block: &Block, indent: usize) {
    let prefix = "  ".repeat(indent);

    match block {
        Block::Heading(h) => {
            println!("{}Content: {}", prefix, format_inlines(&h.content));
        }
        Block::Paragraph(p) => {
            println!("{}Content: {}", prefix, format_inlines(&p.content));
        }
        Block::List(l) => {
            for (i, item) in l.items.iter().enumerate() {
                println!("{}Item {}:", prefix, i + 1);
                for block in &item.blocks {
                    print_block_verbose(block, indent + 1);
                }
            }
        }
        Block::CodeBlock(c) => {
            let preview: String = c.content.chars().take(60).collect();
            let ellipsis = if c.content.len() > 60 { "..." } else { "" };
            println!(
                "{}Content: {}{}",
                prefix,
                preview.replace('\n', "\\n"),
                ellipsis
            );
        }
        Block::Callout(c) => {
            for (i, block) in c.blocks.iter().enumerate() {
                println!("{}Block {}:", prefix, i + 1);
                print_block_verbose(block, indent + 1);
            }
        }
        Block::Quote(q) => {
            for (i, block) in q.blocks.iter().enumerate() {
                println!("{}Block {}:", prefix, i + 1);
                print_block_verbose(block, indent + 1);
            }
        }
        Block::Table(t) => {
            for (i, row) in t.rows.iter().enumerate() {
                let header_marker = if row.header { " (header)" } else { "" };
                let cells: Vec<String> = row
                    .cells
                    .iter()
                    .map(|c| format_inlines(&c.content))
                    .collect();
                println!(
                    "{}Row {}{}: {}",
                    prefix,
                    i + 1,
                    header_marker,
                    cells.join(" | ")
                );
            }
        }
        Block::Footnotes(f) => {
            for def in &f.defs {
                println!("{}[^{}]:", prefix, def.label);
                for block in &def.blocks {
                    print_block_verbose(block, indent + 1);
                }
            }
        }
        Block::Math(m) => {
            let preview: String = m.content.chars().take(40).collect();
            println!("{}Content: {}", prefix, preview);
        }
        _ => {}
    }
}

fn format_inlines(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => result.push_str(&t.content),
            Inline::Emphasis(e) => {
                result.push('*');
                result.push_str(&format_inlines(&e.content));
                result.push('*');
            }
            Inline::Strong(s) => {
                result.push_str("**");
                result.push_str(&format_inlines(&s.content));
                result.push_str("**");
            }
            Inline::CodeSpan(c) => {
                result.push('`');
                result.push_str(&c.content);
                result.push('`');
            }
            Inline::Link(l) => {
                result.push_str("[[");
                result.push_str(&format_inlines(&l.label));
                result.push('|');
                result.push_str(&l.url);
                result.push_str("]]");
            }
            Inline::AutoLink(a) => {
                result.push('<');
                result.push_str(&a.url);
                result.push('>');
            }
            Inline::Strikethrough(s) => {
                result.push_str("~~");
                result.push_str(&format_inlines(&s.content));
                result.push_str("~~");
            }
            Inline::FootnoteRef(f) => {
                result.push_str("[^");
                result.push_str(&f.label);
                result.push(']');
            }
            Inline::HardBreak(_) => result.push_str("\\n"),
            Inline::SoftBreak(_) => result.push(' '),
        }
    }
    result
}

fn format_attr_value(value: &ast::AttrValue) -> String {
    match value {
        ast::AttrValue::Str(s) => format!("\"{}\"", s),
        ast::AttrValue::Bool(b) => b.to_string(),
        ast::AttrValue::Int(i) => i.to_string(),
        ast::AttrValue::Float(f) => f.to_string(),
        ast::AttrValue::List(items) => {
            let formatted: Vec<String> = items.iter().map(format_attr_value).collect();
            format!("[{}]", formatted.join(", "))
        }
    }
}
