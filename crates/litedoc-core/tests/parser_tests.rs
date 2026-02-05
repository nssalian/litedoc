//! Integration tests for the LiteDoc parser

use litedoc_core::ast::{AttrValue, ListKind, Module};
use litedoc_core::{Block, Inline, Parser, Profile};

// ============================================================================
// Profile and Module Directive Tests
// ============================================================================

#[test]
fn test_parse_profile_directive_litedoc() {
    let input = "@profile litedoc\n\n# Hello";
    let mut parser = Parser::new(Profile::Md);
    let doc = parser.parse(input).unwrap();
    assert_eq!(doc.profile, Profile::Litedoc);
}

#[test]
fn test_parse_profile_directive_md() {
    let input = "@profile md\n\n# Hello";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();
    assert_eq!(doc.profile, Profile::Md);
}

#[test]
fn test_parse_profile_directive_md_strict() {
    let input = "@profile md-strict\n\n# Hello";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();
    assert_eq!(doc.profile, Profile::MdStrict);
}

#[test]
fn test_parse_modules_directive() {
    let input = "@modules tables, footnotes, math\n\n# Hello";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();
    assert_eq!(doc.modules.len(), 3);
    assert!(doc.modules.contains(&Module::Tables));
    assert!(doc.modules.contains(&Module::Footnotes));
    assert!(doc.modules.contains(&Module::Math));
}

#[test]
fn test_parse_all_modules() {
    let input = "@modules tables, footnotes, math, tasks, strikethrough, autolink, html\n\n# Hello";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();
    assert_eq!(doc.modules.len(), 7);
}

// ============================================================================
// Metadata Block Tests
// ============================================================================

#[test]
fn test_parse_metadata_basic() {
    let input = "--- meta\ntitle: Hello World\nauthor: John Doe\n---\n\n# Content";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert!(doc.metadata.is_some());
    let meta = doc.metadata.unwrap();
    assert_eq!(meta.entries.len(), 2);
}

#[test]
fn test_parse_metadata_types() {
    let input = r#"--- meta
title: "Hello"
count: 42
price: 19.99
enabled: true
disabled: false
tags: [one, two, three]
---

# Content"#;
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    let meta = doc.metadata.unwrap();

    for (key, value) in &meta.entries {
        match key.as_ref() {
            "title" => assert!(matches!(value, AttrValue::Str(_))),
            "count" => assert!(matches!(value, AttrValue::Int(42))),
            "price" => assert!(matches!(value, AttrValue::Float(f) if (*f - 19.99).abs() < 0.001)),
            "enabled" => assert!(matches!(value, AttrValue::Bool(true))),
            "disabled" => assert!(matches!(value, AttrValue::Bool(false))),
            "tags" => assert!(matches!(value, AttrValue::List(items) if items.len() == 3)),
            _ => panic!("Unexpected key: {}", key),
        }
    }
}

// ============================================================================
// Heading Tests
// ============================================================================

#[test]
fn test_parse_heading_levels() {
    let input = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 6);

    for (i, block) in doc.blocks.iter().enumerate() {
        if let Block::Heading(h) = block {
            assert_eq!(h.level, (i + 1) as u8);
        } else {
            panic!("Expected heading, got {:?}", block);
        }
    }
}

#[test]
fn test_parse_heading_content() {
    let input = "# Hello **World**";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Heading(h) = &doc.blocks[0] {
        assert_eq!(h.level, 1);
        assert_eq!(h.content.len(), 2);
    } else {
        panic!("Expected heading");
    }
}

#[test]
fn test_invalid_heading_no_space() {
    let input = "#NoSpace";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    // Should be parsed as paragraph, not heading
    assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
}

#[test]
fn test_heading_level_too_high() {
    let input = "####### Seven hashes";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    // Should be parsed as paragraph
    assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
}

// ============================================================================
// Paragraph Tests
// ============================================================================

#[test]
fn test_parse_simple_paragraph() {
    let input = "Hello, world!";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 1);
    if let Block::Paragraph(p) = &doc.blocks[0] {
        assert_eq!(p.content.len(), 1);
        if let Inline::Text(t) = &p.content[0] {
            assert_eq!(t.content.as_ref(), "Hello, world!");
        }
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_multiline_paragraph() {
    let input = "Line one\nLine two\nLine three";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 1);
    assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
}

#[test]
fn test_parse_multiple_paragraphs() {
    let input = "First paragraph.\n\nSecond paragraph.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 2);
}

// ============================================================================
// Code Block Tests
// ============================================================================

#[test]
fn test_parse_code_block() {
    let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 1);
    if let Block::CodeBlock(c) = &doc.blocks[0] {
        assert_eq!(c.lang.as_ref(), "rust");
        assert!(c.content.contains("fn main()"));
    } else {
        panic!("Expected code block");
    }
}

#[test]
fn test_parse_code_block_no_lang() {
    let input = "```\nplain code\n```";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::CodeBlock(c) = &doc.blocks[0] {
        assert!(c.lang.is_empty());
    } else {
        panic!("Expected code block");
    }
}

// ============================================================================
// List Block Tests
// ============================================================================

#[test]
fn test_parse_unordered_list() {
    let input = "::list\n- Item one\n- Item two\n- Item three\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::List(l) = &doc.blocks[0] {
        assert_eq!(l.kind, ListKind::Unordered);
        assert_eq!(l.items.len(), 3);
    } else {
        panic!("Expected list block");
    }
}

#[test]
fn test_parse_ordered_list() {
    let input = "::list ordered\n- First\n- Second\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::List(l) = &doc.blocks[0] {
        assert_eq!(l.kind, ListKind::Ordered);
    } else {
        panic!("Expected list block");
    }
}

#[test]
fn test_parse_ordered_list_with_start() {
    let input = "::list ordered start=5\n- Fifth\n- Sixth\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::List(l) = &doc.blocks[0] {
        assert_eq!(l.kind, ListKind::Ordered);
        assert_eq!(l.start, Some(5));
    } else {
        panic!("Expected list block");
    }
}

// ============================================================================
// Callout Block Tests
// ============================================================================

#[test]
fn test_parse_callout() {
    let input = "::callout type=\"warning\" title=\"Be Careful\"\nThis is important.\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Callout(c) = &doc.blocks[0] {
        assert_eq!(c.kind.as_ref(), "warning");
        assert_eq!(c.title.as_ref().map(|s| s.as_ref()), Some("Be Careful"));
    } else {
        panic!("Expected callout block");
    }
}

#[test]
fn test_parse_callout_default_type() {
    let input = "::callout\nDefault note callout.\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Callout(c) = &doc.blocks[0] {
        assert_eq!(c.kind.as_ref(), "note");
    } else {
        panic!("Expected callout block");
    }
}

// ============================================================================
// Quote Block Tests
// ============================================================================

#[test]
fn test_parse_quote() {
    let input = "::quote\nTo be or not to be.\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert!(matches!(&doc.blocks[0], Block::Quote(_)));
}

// ============================================================================
// Figure Block Tests
// ============================================================================

#[test]
fn test_parse_figure() {
    let input = "::figure src=\"image.png\" alt=\"An image\" caption=\"Figure 1\"\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Figure(f) = &doc.blocks[0] {
        assert_eq!(f.src.as_ref(), "image.png");
        assert_eq!(f.alt.as_ref(), "An image");
        assert_eq!(f.caption.as_ref().map(|s| s.as_ref()), Some("Figure 1"));
    } else {
        panic!("Expected figure block");
    }
}

// ============================================================================
// Table Block Tests
// ============================================================================

#[test]
fn test_parse_table() {
    let input = "::table\n| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Table(t) = &doc.blocks[0] {
        assert_eq!(t.rows.len(), 3); // header + 2 data rows
        assert!(t.rows[0].header);
        assert!(!t.rows[1].header);
    } else {
        panic!("Expected table block");
    }
}

// ============================================================================
// Footnotes Block Tests
// ============================================================================

#[test]
fn test_parse_footnotes() {
    let input = "::footnotes\n[^1]: First footnote.\n[^2]: Second footnote.\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Footnotes(f) = &doc.blocks[0] {
        assert_eq!(f.defs.len(), 2);
        assert_eq!(f.defs[0].label.as_ref(), "1");
        assert_eq!(f.defs[1].label.as_ref(), "2");
    } else {
        panic!("Expected footnotes block");
    }
}

// ============================================================================
// Math Block Tests
// ============================================================================

#[test]
fn test_parse_math_block() {
    let input = "::math display\nE = mc^2\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Math(m) = &doc.blocks[0] {
        assert!(m.display);
        assert!(m.content.contains("E = mc^2"));
    } else {
        panic!("Expected math block");
    }
}

#[test]
fn test_parse_math_inline() {
    let input = "::math\nx + y = z\n::";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Math(m) = &doc.blocks[0] {
        assert!(!m.display);
    } else {
        panic!("Expected math block");
    }
}

// ============================================================================
// Thematic Break Tests
// ============================================================================

#[test]
fn test_parse_thematic_break() {
    let input = "Paragraph one.\n\n---\n\nParagraph two.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 3);
    assert!(matches!(&doc.blocks[1], Block::ThematicBreak(_)));
}

// ============================================================================
// Inline Element Tests
// ============================================================================

#[test]
fn test_parse_inline_emphasis() {
    let input = "This is *emphasized* text.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_emphasis = p.content.iter().any(|i| matches!(i, Inline::Emphasis(_)));
        assert!(has_emphasis);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_strong() {
    let input = "This is **strong** text.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_strong = p.content.iter().any(|i| matches!(i, Inline::Strong(_)));
        assert!(has_strong);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_code_span() {
    let input = "Use the `println!` macro.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_code = p.content.iter().any(|i| {
            if let Inline::CodeSpan(c) = i {
                c.content.as_ref() == "println!"
            } else {
                false
            }
        });
        assert!(has_code);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_link() {
    let input = "Visit [[Example|https://example.com]] for more.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_link = p.content.iter().any(|i| {
            if let Inline::Link(l) = i {
                l.url.as_ref() == "https://example.com"
            } else {
                false
            }
        });
        assert!(has_link);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_autolink() {
    let input = "Check <https://example.com> for details.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_autolink = p.content.iter().any(|i| matches!(i, Inline::AutoLink(_)));
        assert!(has_autolink);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_strikethrough() {
    let input = "This is ~~deleted~~ text.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_strike = p
            .content
            .iter()
            .any(|i| matches!(i, Inline::Strikethrough(_)));
        assert!(has_strike);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_inline_footnote_ref() {
    let input = "This has a footnote[^1] reference.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_fn_ref = p.content.iter().any(|i| {
            if let Inline::FootnoteRef(f) = i {
                f.label.as_ref() == "1"
            } else {
                false
            }
        });
        assert!(has_fn_ref);
    } else {
        panic!("Expected paragraph");
    }
}

#[test]
fn test_parse_nested_inline() {
    let input = "This is **bold with *italic* inside**.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    if let Block::Paragraph(p) = &doc.blocks[0] {
        let has_strong = p.content.iter().any(|i| {
            if let Inline::Strong(s) = i {
                s.content
                    .iter()
                    .any(|inner| matches!(inner, Inline::Emphasis(_)))
            } else {
                false
            }
        });
        assert!(has_strong);
    } else {
        panic!("Expected paragraph");
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_parse_empty_input() {
    let input = "";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 0);
}

#[test]
fn test_parse_whitespace_only() {
    let input = "   \n\n   \n";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 0);
}

#[test]
fn test_parse_unclosed_code_block() {
    let input = "```rust\nfn main() {}";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    // Parser should still produce a code block
    assert_eq!(doc.blocks.len(), 1);
    assert!(matches!(&doc.blocks[0], Block::CodeBlock(_)));
}

#[test]
fn test_parse_unclosed_fenced_block() {
    let input = "::callout\nThis is never closed.";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    // Should produce a callout with the content
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_span_tracking() {
    let input = "# Hello";
    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.span.start, 0);
    assert_eq!(doc.span.end, input.len() as u32);

    if let Block::Heading(h) = &doc.blocks[0] {
        assert_eq!(h.span.start, 0);
        assert_eq!(h.span.end, 7);
    }
}

// ============================================================================
// Complex Document Tests
// ============================================================================

#[test]
fn test_parse_complex_document() {
    let input = r#"@profile litedoc
@modules tables, footnotes

--- meta
title: Test Document
version: 1
---

# Introduction

This is a **complex** document with *multiple* features.

::list
- First item
- Second item
::

::callout type="info"
Important information here.
::

```rust
fn example() {}
```

---

## Conclusion

Final paragraph with a [[link|https://example.com]].
"#;

    let mut parser = Parser::new(Profile::Litedoc);
    let doc = parser.parse(input).unwrap();

    assert_eq!(doc.profile, Profile::Litedoc);
    assert_eq!(doc.modules.len(), 2);
    assert!(doc.metadata.is_some());
    assert!(doc.blocks.len() >= 7);
}
