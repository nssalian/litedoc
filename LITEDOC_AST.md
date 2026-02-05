# LiteDoc AST v0.1

This document defines the canonical AST for LiteDoc parsers, intended for stable
round-trip formatting and fast, zero-copy parsing.

## Design goals

- Deterministic structure and node typing.
- Source spans for every node.
- Minimal allocations (string slices into source where possible).
- Stable serialization for testing and tooling.

## Core types

### Span

```
Span { start: u32, end: u32 }
```

- `start` and `end` are byte offsets in the original UTF-8 source.
- `start` is inclusive, `end` is exclusive.

### AttrMap

```
AttrMap: Vec<(Key, Value)>
Key: Cow<'a, str>
Value: AttrValue
```

```
AttrValue:
  - Str(Cow<'a, str>)
  - Bool(bool)
  - Int(i64)
  - Float(f64)
  - List(Vec<AttrValue>)
```

Attributes are ordered and may appear multiple times (last one wins on lookup).

## Document

```
Document {
  profile: Profile,
  modules: Vec<Module>,
  metadata: Option<Metadata>,
  blocks: Vec<Block>,
  span: Span,
}
```

### Profile

```
Profile: Litedoc | Md | MdStrict
```

### Module

```
Module: Tables | Footnotes | Math | Tasks | Strikethrough | Autolink | Html
```

### Metadata

```
Metadata {
  map: Vec<(Key, AttrValue)>,
  span: Span,
}
```

## Block nodes

```
Block:
  - Heading(Heading)
  - Paragraph(Paragraph)
  - List(List)
  - CodeBlock(CodeBlock)
  - Callout(Callout)
  - Quote(Quote)
  - Figure(Figure)
  - Table(Table)
  - Footnotes(Footnotes)
  - Math(MathBlock)
  - ThematicBreak(ThematicBreak)
  - Html(HtmlBlock)   // only if module html enabled
  - Raw(RawBlock)     // for error recovery
```

### Heading

```
Heading {
  level: u8,
  content: Vec<Inline>,
  span: Span,
}
```

### Paragraph

```
Paragraph {
  content: Vec<Inline>,
  span: Span,
}
```

### List

```
List {
  kind: ListKind,
  start: Option<u64>,
  items: Vec<ListItem>,
  span: Span,
}

ListKind: Ordered | Unordered
```

### ListItem

```
ListItem {
  blocks: Vec<Block>,
  span: Span,
}
```

### CodeBlock

```
CodeBlock {
  lang: Cow<'a, str>,
  content: Cow<'a, str>,
  span: Span,
}
```

### Callout

```
Callout {
  kind: Cow<'a, str>,
  title: Option<Cow<'a, str>>,
  blocks: Vec<Block>,
  span: Span,
}
```

### Quote

```
Quote {
  blocks: Vec<Block>,
  span: Span,
}
```

### Figure

```
Figure {
  src: Cow<'a, str>,
  alt: Cow<'a, str>,
  caption: Option<Cow<'a, str>>,
  span: Span,
}
```

### Table

```
Table {
  rows: Vec<TableRow>,
  span: Span,
}

TableRow {
  cells: Vec<TableCell>,
  header: bool,
  span: Span,
}

TableCell {
  content: Vec<Inline>,
  span: Span,
}
```

### Footnotes

```
Footnotes {
  defs: Vec<FootnoteDef>,
  span: Span,
}

FootnoteDef {
  label: Cow<'a, str>,
  blocks: Vec<Block>,
  span: Span,
}
```

### MathBlock

```
MathBlock {
  display: bool,
  content: Cow<'a, str>,
  span: Span,
}
```

### ThematicBreak

```
ThematicBreak { span: Span }
```

### HtmlBlock

```
HtmlBlock { content: Cow<'a, str>, span: Span }
```

### RawBlock

```
RawBlock { content: Cow<'a, str>, span: Span }
```

## Inline nodes

```
Inline:
  - Text(Text)
  - Emphasis(Emphasis)
  - Strong(Strong)
  - CodeSpan(CodeSpan)
  - Link(Link)
  - AutoLink(AutoLink)
  - Strikethrough(Strikethrough)
  - FootnoteRef(FootnoteRef)
  - HardBreak(HardBreak)
  - SoftBreak(SoftBreak)
```

### Text

```
Text { content: Cow<'a, str>, span: Span }
```

### Emphasis / Strong / Strikethrough

```
Emphasis { content: Vec<Inline>, span: Span }
Strong { content: Vec<Inline>, span: Span }
Strikethrough { content: Vec<Inline>, span: Span }
```

### CodeSpan

```
CodeSpan { content: Cow<'a, str>, span: Span }
```

### Link

```
Link {
  label: Vec<Inline>,
  url: Cow<'a, str>,
  title: Option<Cow<'a, str>>,
  span: Span,
}
```

### AutoLink

```
AutoLink { url: Cow<'a, str>, span: Span }
```

### FootnoteRef

```
FootnoteRef { label: Cow<'a, str>, span: Span }
```

### Breaks

```
HardBreak { span: Span }
SoftBreak { span: Span }
```

## Error handling

- Parsers should emit `RawBlock` for unparseable block regions in recovery mode.
- Inline errors should be captured as literal `Text` nodes.

## Canonical serialization (for tests)

- JSON with stable field order.
- `Span` as `[start, end]` arrays.
- `AttrValue` as JSON primitives or arrays.

## Notes for Rust implementation

- Prefer `Cow<'a, str>` for content slices.
- Use an arena (e.g., `bumpalo`) for `Vec` allocations and node storage.
- Expose both borrowed and owned AST representations if needed by callers.
