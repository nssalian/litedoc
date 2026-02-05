"""
LiteDoc - Deterministic document parser for AI agents and LLM output.

Example:
    >>> import pyld
    >>> doc = pyld.parse("# Hello\\n\\nWorld")
    >>> len(doc.blocks)
    2
"""

from pyld.pyld import (
    # Core
    parse,
    parse_with_recovery,
    Parser,
    Profile,
    ModuleKind,
    Span,
    # Document
    Document,
    ParseResult,
    # Errors
    ParseError,
    ParseErrorKind,
    # Blocks
    Heading,
    Paragraph,
    List,
    ListItem,
    ListKind,
    CodeBlock,
    Callout,
    Quote,
    Figure,
    Table,
    TableRow,
    TableCell,
    Footnotes,
    FootnoteDef,
    MathBlock,
    ThematicBreak,
    HtmlBlock,
    RawBlock,
    # Inlines
    Text,
    Emphasis,
    Strong,
    Strikethrough,
    CodeSpan,
    Link,
    AutoLink,
    FootnoteRef,
    HardBreak,
    SoftBreak,
)

__all__ = [
    # Core
    "parse",
    "parse_with_recovery",
    "Parser",
    "Profile",
    "ModuleKind",
    "Span",
    # Document
    "Document",
    "ParseResult",
    # Errors
    "ParseError",
    "ParseErrorKind",
    # Blocks
    "Heading",
    "Paragraph",
    "List",
    "ListItem",
    "ListKind",
    "CodeBlock",
    "Callout",
    "Quote",
    "Figure",
    "Table",
    "TableRow",
    "TableCell",
    "Footnotes",
    "FootnoteDef",
    "MathBlock",
    "ThematicBreak",
    "HtmlBlock",
    "RawBlock",
    # Inlines
    "Text",
    "Emphasis",
    "Strong",
    "Strikethrough",
    "CodeSpan",
    "Link",
    "AutoLink",
    "FootnoteRef",
    "HardBreak",
    "SoftBreak",
]

__version__ = "0.1.0"
