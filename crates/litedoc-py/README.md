# pyld

[![PyPI](https://img.shields.io/pypi/v/litedoc-py.svg)](https://pypi.org/project/litedoc-py/)
[![Python](https://img.shields.io/pypi/pyversions/pyld.svg)](https://pypi.org/project/litedoc-py/)

Deterministic document parser for AI agents and LLM output. Python bindings for the Rust [litedoc-core](https://crates.io/crates/litedoc-core) library.

## Install

```bash
pip install litedoc-py
```

## Usage

```python
import pyld

# Parse a document
doc = pyld.parse("""
# Hello World

This is a **paragraph** with *formatting*.

::list
- First item
- Second item
::
""")

# Iterate over blocks
for block in doc.blocks:
    match block:
        case pyld.Heading(level=level, content=content):
            print(f"H{level}: {content}")
        case pyld.Paragraph(content=content):
            print(f"Paragraph: {content}")
        case pyld.List(items=items):
            print(f"List with {len(items)} items")

# Parse with error recovery
result = pyld.parse_with_recovery("""
::list
- Unclosed list
""")

print(f"Blocks: {len(result.document.blocks)}")
print(f"Errors: {len(result.errors)}")
print(f"OK: {result.ok}")
```

## API

### Functions

- `parse(input, profile=None)` - Parse a string, raises `ValueError` on error
- `parse_with_recovery(input, profile=None)` - Parse with error recovery, always returns a result

### Classes

- `Parser(profile=None)` - Reusable parser instance
- `Document` - Parsed document with `blocks`, `metadata`, `profile`
- `ParseResult` - Result with `document`, `errors`, `ok`

### Profiles

- `Profile.Litedoc` - Full LiteDoc syntax (default)
- `Profile.Md` - CommonMark + GFM
- `Profile.MdStrict` - Strict CommonMark

### Block Types

`Heading`, `Paragraph`, `List`, `CodeBlock`, `Callout`, `Quote`, `Figure`, `Table`, `Footnotes`, `MathBlock`, `ThematicBreak`, `HtmlBlock`, `RawBlock`

### Inline Types

`Text`, `Emphasis`, `Strong`, `Strikethrough`, `CodeSpan`, `Link`, `AutoLink`, `FootnoteRef`, `HardBreak`, `SoftBreak`

## License

Apache-2.0
