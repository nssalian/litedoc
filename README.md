# LiteDoc

[![CI](https://github.com/nssalian/litedoc/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/nssalian/litedoc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/litedoc-core.svg)](https://crates.io/crates/litedoc-core)
[![docs.rs](https://docs.rs/litedoc-core/badge.svg)](https://docs.rs/litedoc-core)

Deterministic document format for AI agents and LLM output. Explicit block fencing, zero-copy parsing, and error recovery.

## Why LiteDoc?

Markdown is ambiguous. Indentation rules vary, edge cases abound, and parsers disagree.[1] LiteDoc uses explicit `::block` fencing for deterministic parsing that recovers gracefully from malformed input, which is ideal for machine-generated content in LLM pipelines where output parsing can fail when formatting is off.[2]

## Status

v0.1.0 - Initial release with Rust parser and CLI. APIs are intended to be stable within v0.1 but may evolve.

## Stability & Compatibility

LiteDoc follows semantic versioning. For the v0.1 line, we will not make breaking
changes to the core format or public APIs without a version bump and a migration
note in the changelog.

| Document | Description |
|----------|-------------|
| [LITEDOC_SPEC.md](LITEDOC_SPEC.md) | Language specification |
| [LITEDOC_AST.md](LITEDOC_AST.md) | AST reference |

## Performance

| Metric | LiteDoc | Markdown | Improvement |
|--------|---------|----------|-------------|
| Parse speed | 5.451 µs | 5.660 µs | 3.7% faster |
| Inline parsing | 490 ns | 1.313 µs | 63% faster |
| Error recovery | 0.89 | 0.67 | 33% better |

## Install

```bash
cargo add litedoc-core            # Rust library
pip install litedoc-py                  # Python library
cargo install litedoc-cli          # CLI tool
```

## Usage

### Rust

```rust
use litedoc_core::{Block, Parser, Profile};

let mut parser = Parser::new(Profile::Litedoc);
let result = parser.parse_with_recovery(input);

for block in &result.document.blocks {
    match block {
        Block::List(list) => process_list(list),
        Block::Table(table) => process_table(table),
        _ => {}
    }
}
```

### Python

```python
import pyld

doc = pyld.parse("# Hello\n\nWorld")
for block in doc.blocks:
    match block:
        case pyld.Heading(level=level):
            print(f"H{level}")
        case pyld.Paragraph(content=content):
            print(content)
```

## CLI

```bash
ldcli agent_output.ld            # Parse and display structure
ldcli -j agent_output.ld         # Output as JSON
ldcli validate agent_output.ld   # Check for errors
ldcli stats agent_output.ld      # Show statistics
```

## Format

```text
::list
- First item
- Second item
::

::quote
Quoted text
::

::table
| A | B |
|---|---|
| 1 | 2 |
::
```

Metadata:

```text
--- meta ---
agent: summarizer-v2
task_id: abc123
timestamp: 1704067200
confidence: 0.92
tags: [summary, final]
---
```

## Benchmarks

```bash
cargo bench -p litedoc-core
```

```bash
cargo test -p litedoc-core robustness_report -- --nocapture
```

CSV output:

```bash
ROBUSTNESS_CSV=1 cargo test -p litedoc-core robustness_report -- --nocapture
ROBUSTNESS_BENCH_CSV=1 cargo bench -p litedoc-core robustness_benchmark -- --nocapture
```

## References

[1] CommonMark Spec, “Why is a spec needed?” (notes original Markdown syntax is not unambiguous and implementations diverged).
https://spec.commonmark.org/0.31.2/

[2] LangChain docs: OUTPUT_PARSING_FAILURE (example of JSON-in-Markdown parsing failures).
https://docs.langchain.com/oss/python/langchain/errors/OUTPUT_PARSING_FAILURE

## License

Apache-2.0
