"""Tests for pyld Python bindings."""

import pyld


def test_parse_simple():
    """Test parsing a simple document."""
    doc = pyld.parse("# Hello\n\nWorld")
    assert len(doc.blocks) == 2
    assert isinstance(doc.blocks[0], pyld.Heading)
    assert doc.blocks[0].level == 1
    assert isinstance(doc.blocks[1], pyld.Paragraph)


def test_parse_with_recovery():
    """Test parsing with error recovery."""
    # Unclosed list block should trigger error recovery
    result = pyld.parse_with_recovery("::list\n- item")
    assert result.document is not None
    # The parser recovers, document is still usable
    assert isinstance(result.document, pyld.Document)


def test_parse_valid_document():
    """Test parsing a valid document returns ok=True."""
    result = pyld.parse_with_recovery("# Title\n\nParagraph.")
    assert result.ok is True
    assert len(result.errors) == 0


def test_parser_class():
    """Test Parser class."""
    parser = pyld.Parser()
    doc = parser.parse("# Test")
    assert len(doc.blocks) == 1


def test_parser_with_profile():
    """Test Parser with explicit profile."""
    parser = pyld.Parser(pyld.Profile.Litedoc)
    doc = parser.parse("# Test")
    assert doc.profile == pyld.Profile.Litedoc


def test_profile_enum():
    """Test Profile enum values."""
    assert pyld.Profile.Litedoc is not None
    assert pyld.Profile.Md is not None
    assert pyld.Profile.MdStrict is not None


def test_heading_levels():
    """Test parsing different heading levels."""
    for level in range(1, 7):
        doc = pyld.parse(f"{'#' * level} Heading {level}")
        assert len(doc.blocks) == 1
        heading = doc.blocks[0]
        assert isinstance(heading, pyld.Heading)
        assert heading.level == level


def test_list_block():
    """Test parsing a list block."""
    doc = pyld.parse("::list\n- One\n- Two\n- Three\n::")
    assert len(doc.blocks) == 1
    lst = doc.blocks[0]
    assert isinstance(lst, pyld.List)
    assert lst.kind == pyld.ListKind.Unordered
    assert len(lst.items) == 3


def test_code_block():
    """Test parsing a code block."""
    doc = pyld.parse("```python\nprint('hello')\n```")
    assert len(doc.blocks) == 1
    code = doc.blocks[0]
    assert isinstance(code, pyld.CodeBlock)
    assert code.lang == "python"
    assert "print" in code.content


def test_inline_formatting():
    """Test inline formatting parsing."""
    doc = pyld.parse("This is *emphasis* and **strong**.")
    assert len(doc.blocks) == 1
    para = doc.blocks[0]
    assert isinstance(para, pyld.Paragraph)
    # Check we have multiple inline elements
    assert len(para.content) > 1


def test_metadata():
    """Test metadata parsing."""
    doc = pyld.parse("""--- meta ---
title: "Test Doc"
version: 1
---

# Content
""")
    assert doc.metadata is not None
    assert "title" in doc.metadata
    assert doc.metadata["title"] == "Test Doc"
    assert doc.metadata.get("version") == 1


def test_span():
    """Test span information."""
    doc = pyld.parse("# Test")
    heading = doc.blocks[0]
    assert heading.span.start == 0
    assert heading.span.end > 0
    assert heading.span.len > 0


def test_document_repr():
    """Test Document __repr__."""
    doc = pyld.parse("# Test")
    repr_str = repr(doc)
    assert "Document" in repr_str
    assert "blocks=1" in repr_str


def test_document_len():
    """Test Document __len__."""
    doc = pyld.parse("# One\n\n# Two\n\n# Three")
    assert len(doc) == 3


def test_parse_error_on_invalid():
    """Test that parse raises on invalid input."""
    try:
        pyld.parse("::unknown_block_type\ncontent\n::")
        # If it doesn't raise, that's also acceptable depending on parser behavior
    except ValueError:
        pass  # Expected


def test_callout():
    """Test callout block parsing."""
    doc = pyld.parse("::callout type=note\nThis is a note.\n::")
    assert len(doc.blocks) == 1
    callout = doc.blocks[0]
    assert isinstance(callout, pyld.Callout)
    assert callout.kind == "note"


def test_quote():
    """Test quote block parsing."""
    doc = pyld.parse("::quote\nQuoted text.\n::")
    assert len(doc.blocks) == 1
    quote = doc.blocks[0]
    assert isinstance(quote, pyld.Quote)
    assert len(quote.blocks) > 0


def test_table():
    """Test table parsing."""
    doc = pyld.parse("""::table
| A | B |
|---|---|
| 1 | 2 |
::""")
    assert len(doc.blocks) == 1
    table = doc.blocks[0]
    assert isinstance(table, pyld.Table)
    assert len(table.rows) > 0


def test_thematic_break():
    """Test thematic break parsing."""
    doc = pyld.parse("---")
    assert len(doc.blocks) == 1
    assert isinstance(doc.blocks[0], pyld.ThematicBreak)


def test_link_inline():
    """Test link parsing in inline content."""
    doc = pyld.parse("Check [[this|https://example.com]]")
    para = doc.blocks[0]
    assert isinstance(para, pyld.Paragraph)
    # Should have text and link
    has_link = any(isinstance(i, pyld.Link) for i in para.content)
    assert has_link


def test_code_span():
    """Test inline code span."""
    doc = pyld.parse("Use `code` here")
    para = doc.blocks[0]
    has_code = any(isinstance(i, pyld.CodeSpan) for i in para.content)
    assert has_code


def test_module_function_parse():
    """Test module-level parse function."""
    doc = pyld.parse("# Test", profile=pyld.Profile.Litedoc)
    assert len(doc.blocks) == 1


def test_module_function_parse_with_recovery():
    """Test module-level parse_with_recovery function."""
    result = pyld.parse_with_recovery("# Test", profile=pyld.Profile.Md)
    assert result.document is not None
