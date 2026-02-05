# LiteDoc v0.1 Specification

LiteDoc is a structured document format for AI agent communication and reliable LLM output parsing.
This specification defines core syntax, parsing rules, AST expectations, and error recovery behavior.

## Goals

- **Unambiguous parsing**: Explicit block boundaries eliminate Markdown's edge cases
- **Error recovery**: Parsers continue after syntax errors, extracting what they can
- **Agent-friendly**: Easy for LLMs to generate, easy for downstream systems to parse
- **Structured metadata**: Typed key-value pairs for agent context (task IDs, confidence scores, etc.)
- **Round-trip stability**: parse → AST → format yields identical output

## Target Use Cases

1. **AI agent output**: When LLMs generate structured responses for programmatic consumption
2. **Agent-to-agent communication**: Passing structured data between agents in a pipeline
3. **Tool results**: Formatting tool/function outputs for LLM consumption
4. **Structured extraction**: Parsing LLM responses into typed data structures

## Token Efficiency Note

LiteDoc's explicit fencing (`::list`, `::quote`, etc.) uses approximately 10-30% more tokens 
than equivalent Markdown syntax. This is an intentional tradeoff:

| | Markdown | LiteDoc |
|---|----------|---------|
| Token cost | Lower | ~15% higher |
| Parse failures | Common | Rare |
| Error recovery | Difficult | Built-in |
| Structure extraction | Regex hacks | Direct AST |

**Use Markdown** for human-authored content where token cost matters.
**Use LiteDoc** for machine-generated content where parsing reliability matters.

## Non-goals

- Human-friendly writing experience (use Markdown for that)
- Full compatibility with all Markdown extensions
- Inline HTML as a formatting mechanism
- Pretty rendering (LiteDoc is for parsing, not display)

## Profiles

LiteDoc supports profiles that define syntax capabilities.

- `litedoc`: native LiteDoc syntax (full).
- `md`: CommonMark core + GFM subset (tables, task lists, strikethrough, autolinks).
- `md-strict`: CommonMark core only, no HTML.

Profiles are declared at the top of the document.

```
@profile litedoc
```

If omitted, parsers should infer `md` for `.md` files and `litedoc` for `.ld` files.

## Modules

Modules enable additional syntax with explicit opt-in.

```
@modules tables, footnotes, math
```

Supported modules in v0.1: `tables`, `footnotes`, `math`, `tasks`, `strikethrough`, `autolink`.

## Document structure

A LiteDoc document is a sequence of blocks. Blocks are separated by one or more blank lines unless
explicitly delimited.

### Metadata

Metadata uses a dedicated block with simple typing and optional quoting.

```
--- meta ---
title: "LiteDoc"
lang: en
draft: false
version: 1
ratio: 1.25
tags: [docs, ai, format]
authors: ["A. Name", "B. Name"]
id: doc:example
---
```

Rules:
- Metadata must be the first non-blank block in the document.
- Keys are ASCII letters, digits, `_`, and `-`.
- Values are UTF-8 strings unless parsed as a number, boolean, or list.
- Booleans: `true` or `false` (lowercase).
- Integers: `[+-]?[0-9]+` (no underscores).
- Floats: `[+-]?[0-9]+.[0-9]+` (no exponents in v0.1).
- Lists use `[item, item]` with items as strings, numbers, or booleans.
- Strings may be unquoted if they contain no `:`, `#`, `[`, `]`, `,`, or leading/trailing spaces.
- Escapes in quoted strings use `\"` and `\\`.

## Block types

### Headings

```
# Heading 1
## Heading 2
### Heading 3
```

Rules:
- `#` through `######` define levels 1-6.
- Require a space after the marker.

### Paragraph

Any block of text not captured by another block.

### Lists

LiteDoc uses explicit list fences to avoid indentation ambiguity.

```
::list ordered start=1
- First
- Second
::
```

```
::list unordered
- Item
- Item
::
```

Rules:
- List items start with `-` and a space.
- Item continuation lines must start with `| ` to stay in the same item.
- A blank line ends the current item unless followed by `| `.
- Nesting uses explicit `::list` blocks inside items.

Example with continuation and nesting:

```
::list unordered
- Item one
| Continued line
| ::list ordered start=1
| - Nested one
| - Nested two
| ::
- Item two
::
```

### Code blocks

```
```lang
code
```
```

Rules:
- `lang` is required in `litedoc` profile.
- In `md` profile, `lang` is optional.

### Callouts

```
::callout type=note title="Heads up"
Text
::
```

### Quotes

```
::quote
Quoted text
::
```

### Figures

```
::figure src="image.png" alt="A diagram" caption="Overview"
::
```

### Tables (module: tables)

```
::table
| Name | Value |
| ---- | ----- |
| A    | 1     |
::
```

LiteDoc permits pipe tables only inside a `::table` block to disambiguate parsing.

### Footnotes (module: footnotes)

```
Text with a footnote.[^1]

::footnotes
[^1]: Footnote text.
::
```

### Math (module: math)

```
::math block
E = mc^2
::
```

Inline math uses `\( ... \)`.

### Task lists (module: tasks)

```
::list unordered
- [ ] todo
- [x] done
::
```

### Horizontal rule

```
---
```

## Inline syntax

LiteDoc inline is minimal and deterministic.

- Emphasis: `*italic*`
- Strong: `**bold**`
- Code: `` `code` ``
- Link: `[[label|https://example.com]]` or `[[https://example.com]]` (implicit label)
- Autolink (module: autolink): `<https://example.com>`
- Strikethrough (module: strikethrough): `~~text~~`

Rules:
- Inline parsing is greedy, left-to-right, and does not backtrack.
- Emphasis cannot span whitespace at both ends.
- Emphasis markers do not open or close inside alphanumeric words.
- Inline markers are not parsed inside code spans.

## Escaping

Use backslash to escape any inline marker.

Example: `\*` yields `*`.

## Markdown compatibility

When `@profile md` is active:

- Parse CommonMark core.
- Enable GFM subset modules: `tables`, `tasks`, `strikethrough`, `autolink`.
- Raw HTML is disabled by default; parsers may enable with `@module html`.

Ambiguity resolution:
- Prefer LiteDoc deterministic rules in cases where CommonMark is ambiguous.
- Emit a warning in `md-strict` mode for ambiguous constructs.

## AST requirements

Parsers must expose:

- Block nodes with type, attributes, and source span.
- Inline nodes with source spans.
- Metadata as a dedicated node.

## Formatting (canonical)

A formatter must:

- Emit `@profile` and `@modules` if present.
- Use explicit LiteDoc blocks when converting from Markdown.
- Preserve heading levels and list ordering.

## File extensions

- `.ld` for LiteDoc.
- `.md` for Markdown compatibility mode.

## Examples

```
@profile litedoc
@modules tables, footnotes

--- meta ---
key: value
lang: en
---

# LiteDoc

::callout type=note title="Token efficient"
Fewer symbols for the same meaning.
::

::list unordered
- Item one
- Item two
::

Text with a footnote.[^1]

::footnotes
[^1]: Footnote text.
::
```

## Open questions

- Should `lang` be mandatory in code blocks for `md` profile?
- Should list item continuation use `| ` or a different marker?
- Should floats allow exponents in v0.2?
