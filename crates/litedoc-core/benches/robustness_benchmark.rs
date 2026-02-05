//! Benchmark accuracy + speed on noisy inputs.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use litedoc_core::{Block, Parser, Profile};
use pulldown_cmark::{Event, Options, Parser as MdParser, Tag, TagEnd};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    headings: u32,
    paragraphs: u32,
    lists: u32,
    list_items: u32,
    tables: u32,
    code_blocks: u32,
}

impl Counts {
    fn total(self) -> u32 {
        self.headings
            + self.paragraphs
            + self.lists
            + self.list_items
            + self.tables
            + self.code_blocks
    }

    fn diff(self, other: Counts) -> u32 {
        (self.headings as i32 - other.headings as i32).unsigned_abs()
            + (self.paragraphs as i32 - other.paragraphs as i32).unsigned_abs()
            + (self.lists as i32 - other.lists as i32).unsigned_abs()
            + (self.list_items as i32 - other.list_items as i32).unsigned_abs()
            + (self.tables as i32 - other.tables as i32).unsigned_abs()
            + (self.code_blocks as i32 - other.code_blocks as i32).unsigned_abs()
    }
}

type MutationList = &'static [&'static str];

struct Lcg {
    state: u64,
}

const SEED: u64 = 0x5eed;
const MAX_VARIANT_LEN: usize = 32_000;
const MAX_MUTATION_STEPS: usize = 3;
const VARIANT_COUNT: usize = 12;

const LITEDOC_REALISTIC_BASE: MutationList = &[
    "drop_fence_end",
    "drop_code_fence_end",
    "drop_callout_end",
    "drop_blank_line",
    "truncate_tail_small",
    "extra_blank_lines",
    "whitespace_noise",
];

const MARKDOWN_REALISTIC_BASE: MutationList = &[
    "drop_code_fence_end",
    "drop_table_bar",
    "drop_blank_line",
    "truncate_tail_small",
    "strip_list_dash",
];

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn choose(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u32() as usize) % max
    }
}

fn apply_mutations(mut input: String, mutations: &[&str], rng: &mut Lcg) -> String {
    let steps = std::cmp::min(MAX_MUTATION_STEPS, mutations.len());
    for _ in 0..steps {
        let pick = rng.choose(mutations.len());
        match mutations[pick] {
            "drop_fence_end" => {
                if let Some(pos) = input.rfind("::\n") {
                    input.replace_range(pos..pos + 3, "");
                }
            }
            "drop_code_fence_end" => {
                if let Some(pos) = input.rfind("```\n") {
                    input.replace_range(pos..pos + 4, "");
                }
            }
            "drop_callout_end" => {
                if let Some(pos) = input.rfind("::callout") {
                    if let Some(end) = input[pos..].find("::\n") {
                        let cut = pos + end;
                        input.replace_range(cut..cut + 3, "");
                    }
                }
            }
            "drop_table_bar" => {
                if let Some(pos) = input.find("|---|---|") {
                    input.replace_range(pos..pos + 9, "---");
                } else if let Some(pos) = input.find("|---|") {
                    input.replace_range(pos..pos + 5, "---");
                }
            }
            "drop_blank_line" => {
                if let Some(pos) = input.find("\n\n") {
                    input.replace_range(pos..pos + 2, "\n");
                }
            }
            "truncate_tail" => {
                let len = input.len();
                if len > 8 {
                    let cut = rng.choose(len / 4).max(1);
                    input.truncate(len - cut);
                }
            }
            "truncate_tail_small" => {
                let len = input.len();
                if len > 16 {
                    let cut = rng.choose(len / 20).max(1);
                    input.truncate(len - cut);
                }
            }
            "strip_list_dash" => {
                if let Some(pos) = input.find("- ") {
                    input.replace_range(pos..pos + 2, "");
                }
            }
            "extra_blank_lines" => {
                input = input.replace("\n\n", "\n\n\n");
            }
            "whitespace_noise" => {
                input = input.replace("::", " ::");
                input = input.replace("\n- ", "\n-  ");
            }
            "typo_block_tag" => {
                if let Some(pos) = input.find("::list") {
                    input.replace_range(pos..pos + 6, "::lits");
                }
            }
            "typo_callout_attr" => {
                if let Some(pos) = input.find("type=") {
                    input.replace_range(pos..pos + 5, "typo=");
                }
            }
            _ => {}
        }
        if input.len() > MAX_VARIANT_LEN {
            input.truncate(MAX_VARIANT_LEN);
            break;
        }
    }
    input
}

fn generate_variants(input: &str, seed: u64, mutations: MutationList) -> Vec<String> {
    let mut rng = Lcg::new(seed);
    let mut variants = Vec::new();
    for _ in 0..VARIANT_COUNT {
        let mutated = apply_mutations(input.to_string(), mutations, &mut rng);
        variants.push(mutated);
    }
    variants
}

fn count_litedoc_blocks(blocks: &[Block], counts: &mut Counts, in_list: bool) {
    for block in blocks {
        match block {
            Block::Heading(_) => counts.headings += 1,
            Block::Paragraph(_) => {
                if !in_list {
                    counts.paragraphs += 1;
                }
            }
            Block::List(list) => {
                counts.lists += 1;
                counts.list_items += list.items.len() as u32;
                for item in &list.items {
                    count_litedoc_blocks(&item.blocks, counts, true);
                }
            }
            Block::CodeBlock(_) => counts.code_blocks += 1,
            Block::Table(_) => counts.tables += 1,
            Block::Callout(callout) => count_litedoc_blocks(&callout.blocks, counts, in_list),
            Block::Quote(quote) => count_litedoc_blocks(&quote.blocks, counts, in_list),
            _ => {}
        }
    }
}

fn count_litedoc(input: &str) -> Counts {
    let mut parser = Parser::new(Profile::Litedoc);
    let result = parser.parse_with_recovery(input);
    let mut counts = Counts::default();
    count_litedoc_blocks(&result.document.blocks, &mut counts, false);
    counts
}

fn count_markdown(input: &str) -> Counts {
    let mut counts = Counts::default();
    let parser = MdParser::new_ext(input, Options::all());
    let mut list_depth = 0u32;
    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { .. } => counts.headings += 1,
                Tag::Paragraph => {
                    if list_depth == 0 {
                        counts.paragraphs += 1;
                    }
                }
                Tag::List(_) => {
                    counts.lists += 1;
                    list_depth += 1;
                }
                Tag::Item => counts.list_items += 1,
                Tag::Table(_) => counts.tables += 1,
                Tag::CodeBlock(_) => counts.code_blocks += 1,
                _ => {}
            },
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::End(_) => {}
            _ => {}
        }
    }
    counts
}

fn accuracy(counts: Counts, expected: Counts) -> f64 {
    let total = expected.total();
    if total == 0 {
        return 1.0;
    }
    let diff = counts.diff(expected);
    let score = (total as f64 - diff as f64) / total as f64;
    score.max(0.0)
}

struct BenchDoc {
    name: String,
    litedoc_input: String,
    markdown_input: String,
    expected: Counts,
}

fn load_example_docs() -> Vec<BenchDoc> {
    let mut docs = Vec::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples");
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(_) => return docs,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ld") {
            continue;
        }
        let litedoc_input = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => stem,
            None => continue,
        };
        let md_path = root.join(format!("{}.md", stem));
        let markdown_input = fs::read_to_string(&md_path).unwrap_or_else(|_| litedoc_input.clone());
        let expected = count_litedoc(&litedoc_input);
        docs.push(BenchDoc {
            name: stem.to_string(),
            litedoc_input,
            markdown_input,
            expected,
        });
    }
    docs
}

fn bench_robustness(c: &mut Criterion) {
    let csv_enabled = std::env::var("ROBUSTNESS_BENCH_CSV").ok().as_deref() == Some("1");
    let csv_path = std::env::var("ROBUSTNESS_BENCH_CSV_PATH").ok();
    let mut csv_rows: Vec<String> = Vec::new();
    if csv_enabled {
        csv_rows.push("doc,format,avg,strict_ok,recovery_ok,variants".to_string());
    }
    let base_litedoc = r#"
# Title

Paragraph text.

::list
- One
- Two
::

```rust
fn main() {}
```

::table
| A | B |
|---|---|
| 1 | 2 |
::
"#;

    let base_markdown = r#"
# Title

Paragraph text.

- One
- Two

```rust
fn main() {}
```

| A | B |
|---|---|
| 1 | 2 |
"#;

    let expected = Counts {
        headings: 1,
        paragraphs: 1,
        lists: 1,
        list_items: 2,
        tables: 1,
        code_blocks: 1,
    };

    let litedoc_variants = generate_variants(base_litedoc, SEED, LITEDOC_REALISTIC_BASE);
    let markdown_variants = generate_variants(base_markdown, SEED, MARKDOWN_REALISTIC_BASE);

    let mut litedoc_sum = 0.0;
    let mut markdown_sum = 0.0;
    for variant in &litedoc_variants {
        litedoc_sum += accuracy(count_litedoc(variant), expected);
    }
    for variant in &markdown_variants {
        markdown_sum += accuracy(count_markdown(variant), expected);
    }
    let litedoc_avg = litedoc_sum / litedoc_variants.len() as f64;
    let markdown_avg = markdown_sum / markdown_variants.len() as f64;

    println!("\nRobustness Benchmark");
    println!("Seed\t0x{:x}", SEED);
    println!("variants\tlitedoc_avg\tmarkdown_avg\tlitedoc_strict\tlitedoc_recovery");
    let mut litedoc_strict_ok = 0u32;
    let mut litedoc_recovery_ok = 0u32;
    for variant in &litedoc_variants {
        let mut parser = Parser::new(Profile::Litedoc);
        if parser.parse(variant).is_ok() {
            litedoc_strict_ok += 1;
        }
        let acc = accuracy(count_litedoc(variant), expected);
        if acc >= 0.9 {
            litedoc_recovery_ok += 1;
        }
    }
    println!(
        "{}\t{:.2}\t{:.2}\t{}/{}\t{}/{}",
        litedoc_variants.len(),
        litedoc_avg,
        markdown_avg,
        litedoc_strict_ok,
        litedoc_variants.len(),
        litedoc_recovery_ok,
        litedoc_variants.len()
    );

    let mut docs = Vec::new();
    docs.push(BenchDoc {
        name: "base".to_string(),
        litedoc_input: base_litedoc.to_string(),
        markdown_input: base_markdown.to_string(),
        expected,
    });
    docs.extend(load_example_docs());

    if !docs.is_empty() {
        let mut agg_litedoc = 0.0;
        let mut agg_markdown = 0.0;
        let mut agg_count = 0u32;
        let mut agg_strict = 0u32;
        let mut agg_recovery = 0u32;
        let mut agg_variants = 0u32;

        for doc in &docs {
            let litedoc_variants =
                generate_variants(&doc.litedoc_input, SEED, LITEDOC_REALISTIC_BASE);
            let markdown_variants =
                generate_variants(&doc.markdown_input, SEED, MARKDOWN_REALISTIC_BASE);
            let mut litedoc_sum = 0.0;
            let mut markdown_sum = 0.0;
            let mut strict_ok = 0u32;
            let mut recovery_ok = 0u32;
            for variant in &litedoc_variants {
                let acc = accuracy(count_litedoc(variant), doc.expected);
                litedoc_sum += acc;
                let mut parser = Parser::new(Profile::Litedoc);
                if parser.parse(variant).is_ok() {
                    strict_ok += 1;
                }
                if acc >= 0.9 {
                    recovery_ok += 1;
                }
            }
            for variant in &markdown_variants {
                markdown_sum += accuracy(count_markdown(variant), doc.expected);
            }
            let litedoc_avg = litedoc_sum / litedoc_variants.len() as f64;
            let markdown_avg = markdown_sum / markdown_variants.len() as f64;
            agg_litedoc += litedoc_avg;
            agg_markdown += markdown_avg;
            agg_count += 1;
            agg_strict += strict_ok;
            agg_recovery += recovery_ok;
            agg_variants += litedoc_variants.len() as u32;
            if csv_enabled {
                csv_rows.push(format!(
                    "{},litedoc,{:.4},{},{},{}",
                    doc.name,
                    litedoc_avg,
                    strict_ok,
                    recovery_ok,
                    litedoc_variants.len()
                ));
                csv_rows.push(format!(
                    "{},markdown,{:.4},,,{}",
                    doc.name,
                    markdown_avg,
                    markdown_variants.len()
                ));
            }
        }

        println!(
            "Aggregate Benchmark\tdocs={}\tlitedoc_avg={:.2}\tmarkdown_avg={:.2}\tlitedoc_strict_rate={:.2}\tlitedoc_recovery_rate={:.2}",
            docs.len(),
            agg_litedoc / agg_count as f64,
            agg_markdown / agg_count as f64,
            agg_strict as f64 / agg_variants as f64,
            agg_recovery as f64 / agg_variants as f64
        );
        if csv_enabled {
            csv_rows.push(format!(
                "aggregate,litedoc,{:.4},{},{},{}",
                agg_litedoc / agg_count as f64,
                agg_strict,
                agg_recovery,
                agg_variants
            ));
            csv_rows.push(format!(
                "aggregate,markdown,{:.4},,,{}",
                agg_markdown / agg_count as f64,
                agg_variants
            ));
        }
    }

    if csv_enabled {
        let path = match csv_path {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("robustness_benchmark.csv"),
        };
        if let Ok(mut file) = fs::File::create(&path) {
            for row in csv_rows {
                let _ = writeln!(file, "{}", row);
            }
            println!("CSV\t{}", path.display());
        } else {
            println!("CSV");
            for row in csv_rows {
                println!("{}", row);
            }
        }
    }

    let mut group = c.benchmark_group("robustness_speed");

    group.throughput(Throughput::Bytes(base_litedoc.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("litedoc", litedoc_variants.len()),
        &litedoc_variants,
        |b, variants| {
            b.iter(|| {
                for variant in variants {
                    let mut parser = Parser::new(Profile::Litedoc);
                    let doc = parser.parse_with_recovery(black_box(variant));
                    black_box(doc.document.blocks.len());
                }
            })
        },
    );

    group.throughput(Throughput::Bytes(base_markdown.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("markdown", markdown_variants.len()),
        &markdown_variants,
        |b, variants| {
            b.iter(|| {
                for variant in variants {
                    let parser = MdParser::new_ext(black_box(variant), Options::all());
                    let events: Vec<_> = parser.collect();
                    black_box(events.len());
                }
            })
        },
    );

    group.finish();
}

criterion_group!(robustness, bench_robustness);
criterion_main!(robustness);
