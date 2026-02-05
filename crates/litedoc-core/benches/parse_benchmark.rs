//! Benchmarks comparing LiteDoc parsing vs pulldown-cmark (Markdown)
//!
//! Run with: cargo bench -p litedoc-core

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use litedoc_core::{Parser, Profile};
use pulldown_cmark::{Options, Parser as MdParser};

/// Sample LiteDoc content
const LITEDOC_SAMPLE: &str = r#"@profile litedoc
@modules tables, footnotes

--- meta ---
title: "Benchmark Document"
version: 1
---

# Introduction

This is a paragraph with *emphasis*, **strong text**, and `inline code`.
It demonstrates the basic capabilities of the format.

## Lists

::list unordered
- First item with some content
- Second item with more content
- Third item concluding the list
::

::list ordered start=1
- Step one of the process
- Step two continues
- Step three completes
::

## Code Example

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

## Callout

::callout type=note title="Performance"
LiteDoc is designed for deterministic, fast parsing.
No backtracking required.
::

## Table

::table
| Name    | Speed   | Memory |
| ------- | ------- | ------ |
| Fast    | 100ms   | 10MB   |
| Medium  | 500ms   | 50MB   |
| Slow    | 1000ms  | 100MB  |
::

## Quote

::quote
The best code is no code at all.
Every line of code you write is a liability.

-- Someone wise
::

---

End of document.
"#;

/// Equivalent Markdown content (as close as possible)
const MARKDOWN_SAMPLE: &str = r#"---
title: "Benchmark Document"
version: 1
---

# Introduction

This is a paragraph with *emphasis*, **strong text**, and `inline code`.
It demonstrates the basic capabilities of the format.

## Lists

- First item with some content
- Second item with more content
- Third item concluding the list

1. Step one of the process
2. Step two continues
3. Step three completes

## Code Example

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

## Callout

> **Note: Performance**
>
> LiteDoc is designed for deterministic, fast parsing.
> No backtracking required.

## Table

| Name    | Speed   | Memory |
| ------- | ------- | ------ |
| Fast    | 100ms   | 10MB   |
| Medium  | 500ms   | 50MB   |
| Slow    | 1000ms  | 100MB  |

## Quote

> The best code is no code at all.
> Every line of code you write is a liability.
>
> -- Someone wise

---

End of document.
"#;

fn bench_litedoc_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    // Set throughput for bytes/sec reporting
    group.throughput(Throughput::Bytes(LITEDOC_SAMPLE.len() as u64));

    group.bench_function("litedoc", |b| {
        b.iter(|| {
            let mut parser = Parser::new(Profile::Litedoc);
            let doc = parser.parse(black_box(LITEDOC_SAMPLE)).unwrap();
            black_box(doc.blocks.len())
        })
    });

    group.throughput(Throughput::Bytes(MARKDOWN_SAMPLE.len() as u64));

    group.bench_function("markdown_pulldown", |b| {
        b.iter(|| {
            let parser = MdParser::new_ext(black_box(MARKDOWN_SAMPLE), Options::all());
            let events: Vec<_> = parser.collect();
            black_box(events.len())
        })
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    // Test with different document sizes
    for size in [1, 5, 10, 20].iter() {
        let litedoc_content: String = LITEDOC_SAMPLE.repeat(*size);
        let markdown_content: String = MARKDOWN_SAMPLE.repeat(*size);

        group.throughput(Throughput::Bytes(litedoc_content.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("litedoc", size),
            &litedoc_content,
            |b, content| {
                b.iter(|| {
                    let mut parser = Parser::new(Profile::Litedoc);
                    let doc = parser.parse(black_box(content)).unwrap();
                    black_box(doc.blocks.len())
                })
            },
        );

        group.throughput(Throughput::Bytes(markdown_content.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("markdown", size),
            &markdown_content,
            |b, content| {
                b.iter(|| {
                    let parser = MdParser::new_ext(black_box(content), Options::all());
                    let events: Vec<_> = parser.collect();
                    black_box(events.len())
                })
            },
        );
    }

    group.finish();
}

fn bench_inline_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("inline");

    let litedoc_inline =
        "This has *emphasis*, **strong**, `code`, [[link|https://example.com]], and ~~strike~~.";
    let markdown_inline =
        "This has *emphasis*, **strong**, `code`, [link](https://example.com), and ~~strike~~.";

    group.bench_function("litedoc_inline", |b| {
        b.iter(|| {
            let inlines = litedoc_core::inline::parse_inlines(black_box(litedoc_inline), 0, "");
            black_box(inlines.len())
        })
    });

    group.bench_function("markdown_inline", |b| {
        b.iter(|| {
            let parser = MdParser::new_ext(black_box(markdown_inline), Options::all());
            let events: Vec<_> = parser.collect();
            black_box(events.len())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_litedoc_parse,
    bench_scaling,
    bench_inline_parsing
);
criterion_main!(benches);
