use litedoc_core::{Block, Parser, Profile};
use pulldown_cmark::{Event, Options, Parser as MdParser, Tag, TagEnd};
use std::fs;
use std::io::{self, Write};
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

#[derive(Debug)]
struct Case<'a> {
    name: &'a str,
    litedoc_input: &'a str,
    markdown_input: &'a str,
    expected: Counts,
    realistic_litedoc: MutationList,
    realistic_markdown: MutationList,
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

type MutationList = &'static [&'static str];

struct Lcg {
    state: u64,
}

const SEED: u64 = 0x5eed;
const MAX_VARIANT_LEN: usize = 32_000;
const MAX_MUTATION_STEPS: usize = 3;
const VARIANT_COUNT: usize = 8;

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

const LITEDOC_REALISTIC_NESTED: MutationList =
    &["drop_blank_line", "truncate_tail_small", "whitespace_noise"];

const MARKDOWN_REALISTIC_NESTED: MutationList =
    &["drop_blank_line", "truncate_tail_small", "strip_list_dash"];

const LITEDOC_REALISTIC_MISSING_CODE: MutationList = &[
    "drop_code_fence_end",
    "drop_blank_line",
    "truncate_tail_small",
    "whitespace_noise",
];

const MARKDOWN_REALISTIC_MISSING_CODE: MutationList = &[
    "drop_code_fence_end",
    "drop_blank_line",
    "truncate_tail_small",
];

const STRESS_MUTATIONS: MutationList = &[
    "drop_fence_end",
    "drop_code_fence_end",
    "drop_table_bar",
    "drop_blank_line",
    "truncate_tail",
    "strip_list_dash",
    "typo_block_tag",
    "typo_callout_attr",
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

fn report_case(case: &Case) {
    let litedoc_counts = count_litedoc(case.litedoc_input);
    let markdown_counts = count_markdown(case.markdown_input);
    let litedoc_acc = accuracy(litedoc_counts, case.expected);
    let markdown_acc = accuracy(markdown_counts, case.expected);

    println!(
        "{}\tlitedoc\t{:.2}\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        litedoc_acc,
        litedoc_counts.headings,
        litedoc_counts.paragraphs,
        litedoc_counts.lists,
        litedoc_counts.list_items,
        litedoc_counts.tables,
        litedoc_counts.code_blocks
    );
    println!(
        "{}\tmarkdown\t{:.2}\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        markdown_acc,
        markdown_counts.headings,
        markdown_counts.paragraphs,
        markdown_counts.lists,
        markdown_counts.list_items,
        markdown_counts.tables,
        markdown_counts.code_blocks
    );
}

fn strict_parse_ok(input: &str) -> bool {
    let mut parser = Parser::new(Profile::Litedoc);
    parser.parse(input).is_ok()
}

#[derive(Clone, Copy)]
struct ReportStats {
    avg: f64,
    min: f64,
    max: f64,
    strict_ok: u32,
    recovery_ok: u32,
    variants: usize,
}

fn report_noisy(
    case: &Case,
    litedoc_mutations: MutationList,
    markdown_mutations: MutationList,
    label: &str,
) -> (ReportStats, ReportStats) {
    let litedoc_variants = generate_variants(case.litedoc_input, SEED, litedoc_mutations);
    let markdown_variants = generate_variants(case.markdown_input, SEED, markdown_mutations);

    let mut litedoc_sum = 0.0;
    let mut markdown_sum = 0.0;
    let mut litedoc_min: f64 = 1.0;
    let mut markdown_min: f64 = 1.0;
    let mut litedoc_max: f64 = 0.0;
    let mut markdown_max: f64 = 0.0;
    let mut litedoc_strict_ok = 0u32;
    let mut litedoc_recovery_ok = 0u32;

    for variant in &litedoc_variants {
        let acc = accuracy(count_litedoc(variant), case.expected);
        litedoc_sum += acc;
        litedoc_min = litedoc_min.min(acc);
        litedoc_max = litedoc_max.max(acc);
        if strict_parse_ok(variant) {
            litedoc_strict_ok += 1;
        }
        if acc >= 0.9 {
            litedoc_recovery_ok += 1;
        }
    }

    for variant in &markdown_variants {
        let acc = accuracy(count_markdown(variant), case.expected);
        markdown_sum += acc;
        markdown_min = markdown_min.min(acc);
        markdown_max = markdown_max.max(acc);
    }

    let litedoc_avg = litedoc_sum / litedoc_variants.len() as f64;
    let markdown_avg = markdown_sum / markdown_variants.len() as f64;

    println!(
        "{}_{}\tlitedoc\t{:.2}\tmin={:.2}\tmax={:.2}\tstrict={}/{}\trecovery={}/{}",
        case.name,
        label,
        litedoc_avg,
        litedoc_min,
        litedoc_max,
        litedoc_strict_ok,
        litedoc_variants.len(),
        litedoc_recovery_ok,
        litedoc_variants.len()
    );
    println!(
        "{}_{}\tmarkdown\t{:.2}\tmin={:.2}\tmax={:.2}",
        case.name, label, markdown_avg, markdown_min, markdown_max
    );

    let litedoc_stats = ReportStats {
        avg: litedoc_avg,
        min: litedoc_min,
        max: litedoc_max,
        strict_ok: litedoc_strict_ok,
        recovery_ok: litedoc_recovery_ok,
        variants: litedoc_variants.len(),
    };
    let markdown_stats = ReportStats {
        avg: markdown_avg,
        min: markdown_min,
        max: markdown_max,
        strict_ok: 0,
        recovery_ok: 0,
        variants: markdown_variants.len(),
    };

    (litedoc_stats, markdown_stats)
}

fn load_corpus_cases() -> Vec<Case<'static>> {
    let mut cases = Vec::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples");
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ld") {
                continue;
            }
            if let Ok(contents) = fs::read_to_string(&path) {
                let expected = count_litedoc(&contents);
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("example")
                    .to_string();
                cases.push(Case {
                    name: Box::leak(name.into_boxed_str()),
                    litedoc_input: Box::leak(contents.clone().into_boxed_str()),
                    markdown_input: Box::leak(contents.into_boxed_str()),
                    expected,
                    realistic_litedoc: LITEDOC_REALISTIC_BASE,
                    realistic_markdown: MARKDOWN_REALISTIC_BASE,
                });
            }
        }
    }
    cases
}

#[test]
fn robustness_report() {
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

    let multiline_litedoc = r#"
# Title

Paragraph text.

::list
- One
| continued line
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

    let multiline_markdown = r#"
# Title

Paragraph text.

- One
continued line
- Two

```rust
fn main() {}
```

| A | B |
|---|---|
| 1 | 2 |
"#;

    let nested_litedoc = r#"
# Title

Paragraph text.

::list
- Parent one
| ::list
| - Child A
| - Child B
| ::
- Parent two
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

    let nested_markdown = r#"
# Title

Paragraph text.

- Parent one
- Child A
- Child B
- Parent two

```rust
fn main() {}
```

| A | B |
|---|---|
| 1 | 2 |
"#;

    let missing_list_fence_litedoc = r#"
# Title

Paragraph text.

::list
- One
- Two

```rust
fn main() {}
```

::table
| A | B |
|---|---|
| 1 | 2 |
::
"#;

    let missing_list_fence_markdown = r#"
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

    let missing_code_fence_litedoc = r#"
# Title

Paragraph text.

::list
- One
- Two
::

```rust
fn main() {}

::table
| A | B |
|---|---|
| 1 | 2 |
::
"#;

    let missing_code_fence_markdown = r#"
# Title

Paragraph text.

- One
- Two

```rust
fn main() {}

| A | B |
|---|---|
| 1 | 2 |
"#;

    let missing_table_fence_litedoc = r#"
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
"#;

    let missing_table_fence_markdown = r#"
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

    let cases = vec![
        Case {
            name: "clean",
            litedoc_input: base_litedoc,
            markdown_input: base_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 1,
                list_items: 2,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_BASE,
            realistic_markdown: MARKDOWN_REALISTIC_BASE,
        },
        Case {
            name: "multiline_list_item",
            litedoc_input: multiline_litedoc,
            markdown_input: multiline_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 1,
                list_items: 2,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_BASE,
            realistic_markdown: MARKDOWN_REALISTIC_BASE,
        },
        Case {
            name: "nested_list_missing_indent",
            litedoc_input: nested_litedoc,
            markdown_input: nested_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 2,
                list_items: 4,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_NESTED,
            realistic_markdown: MARKDOWN_REALISTIC_NESTED,
        },
        Case {
            name: "missing_list_fence",
            litedoc_input: missing_list_fence_litedoc,
            markdown_input: missing_list_fence_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 1,
                list_items: 2,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_BASE,
            realistic_markdown: MARKDOWN_REALISTIC_BASE,
        },
        Case {
            name: "missing_code_fence",
            litedoc_input: missing_code_fence_litedoc,
            markdown_input: missing_code_fence_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 1,
                list_items: 2,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_MISSING_CODE,
            realistic_markdown: MARKDOWN_REALISTIC_MISSING_CODE,
        },
        Case {
            name: "missing_table_fence",
            litedoc_input: missing_table_fence_litedoc,
            markdown_input: missing_table_fence_markdown,
            expected: Counts {
                headings: 1,
                paragraphs: 1,
                lists: 1,
                list_items: 2,
                tables: 1,
                code_blocks: 1,
            },
            realistic_litedoc: LITEDOC_REALISTIC_BASE,
            realistic_markdown: MARKDOWN_REALISTIC_BASE,
        },
    ];

    println!("\nRobustness Report");
    println!("Seed\t0x{:x}", SEED);
    println!("case\tformat\taccuracy\theadings\tparagraphs\tlists\titems\ttables\tcode");
    let mut realistic_sum_litedoc = 0.0;
    let mut realistic_sum_markdown = 0.0;
    let mut realistic_count = 0u32;
    let mut realistic_strict_ok = 0u32;
    let mut realistic_recovery_ok = 0u32;
    let mut realistic_variants = 0u32;
    let mut stress_sum_litedoc = 0.0;
    let mut stress_sum_markdown = 0.0;
    let mut stress_count = 0u32;
    let csv_enabled = std::env::var("ROBUSTNESS_CSV").ok().as_deref() == Some("1");
    let csv_path = std::env::var("ROBUSTNESS_CSV_PATH").ok();
    let mut csv_rows: Vec<String> = Vec::new();
    if csv_enabled {
        csv_rows
            .push("suite,case,label,format,avg,min,max,strict_ok,recovery_ok,variants".to_string());
    }
    for case in &cases {
        report_case(case);
        let (litedoc_stats, markdown_stats) = report_noisy(
            case,
            case.realistic_litedoc,
            case.realistic_markdown,
            "realistic",
        );
        realistic_sum_litedoc += litedoc_stats.avg;
        realistic_sum_markdown += markdown_stats.avg;
        realistic_count += 1;
        realistic_strict_ok += litedoc_stats.strict_ok;
        realistic_recovery_ok += litedoc_stats.recovery_ok;
        realistic_variants += litedoc_stats.variants as u32;
        if csv_enabled {
            csv_rows.push(format!(
                "suite,{},realistic,litedoc,{:.4},{:.4},{:.4},{},{},{}",
                case.name,
                litedoc_stats.avg,
                litedoc_stats.min,
                litedoc_stats.max,
                litedoc_stats.strict_ok,
                litedoc_stats.recovery_ok,
                litedoc_stats.variants
            ));
            csv_rows.push(format!(
                "suite,{},realistic,markdown,{:.4},{:.4},{:.4},,,{}",
                case.name,
                markdown_stats.avg,
                markdown_stats.min,
                markdown_stats.max,
                markdown_stats.variants
            ));
        }
        let (stress_litedoc_stats, stress_markdown_stats) =
            report_noisy(case, STRESS_MUTATIONS, STRESS_MUTATIONS, "stress");
        stress_sum_litedoc += stress_litedoc_stats.avg;
        stress_sum_markdown += stress_markdown_stats.avg;
        stress_count += 1;
        if csv_enabled {
            csv_rows.push(format!(
                "suite,{},stress,litedoc,{:.4},{:.4},{:.4},{},{},{}",
                case.name,
                stress_litedoc_stats.avg,
                stress_litedoc_stats.min,
                stress_litedoc_stats.max,
                stress_litedoc_stats.strict_ok,
                stress_litedoc_stats.recovery_ok,
                stress_litedoc_stats.variants
            ));
            csv_rows.push(format!(
                "suite,{},stress,markdown,{:.4},{:.4},{:.4},,,{}",
                case.name,
                stress_markdown_stats.avg,
                stress_markdown_stats.min,
                stress_markdown_stats.max,
                stress_markdown_stats.variants
            ));
        }
        let _ = io::stdout().flush();
    }
    if realistic_count > 0 {
        println!(
            "Realistic Aggregate\tlitedoc_avg={:.2}\tmarkdown_avg={:.2}\tlitedoc_strict_rate={:.2}\tlitedoc_recovery_rate={:.2}",
            realistic_sum_litedoc / realistic_count as f64,
            realistic_sum_markdown / realistic_count as f64,
            realistic_strict_ok as f64 / realistic_variants as f64,
            realistic_recovery_ok as f64 / realistic_variants as f64
        );
        println!(
            "Executive Summary\tlitedoc_avg={:.2}\tmarkdown_avg={:.2}\tlitedoc_strict_rate={:.2}\tlitedoc_recovery_rate={:.2}",
            realistic_sum_litedoc / realistic_count as f64,
            realistic_sum_markdown / realistic_count as f64,
            realistic_strict_ok as f64 / realistic_variants as f64,
            realistic_recovery_ok as f64 / realistic_variants as f64
        );
    }
    if stress_count > 0 {
        println!(
            "Stress Summary\tlitedoc_avg={:.2}\tmarkdown_avg={:.2}",
            stress_sum_litedoc / stress_count as f64,
            stress_sum_markdown / stress_count as f64
        );
    }

    let corpus_cases = load_corpus_cases();
    if !corpus_cases.is_empty() {
        println!("\nCorpus Report (LiteDoc examples)");
        println!("case\tformat\taccuracy\tstrict\trecovery");
        for case in &corpus_cases {
            let variants = generate_variants(case.litedoc_input, SEED, LITEDOC_REALISTIC_BASE);
            let mut sum = 0.0;
            let mut strict_ok = 0u32;
            let mut recovery_ok = 0u32;
            for variant in &variants {
                let acc = accuracy(count_litedoc(variant), case.expected);
                sum += acc;
                if strict_parse_ok(variant) {
                    strict_ok += 1;
                }
                if acc >= 0.9 {
                    recovery_ok += 1;
                }
            }
            let avg = sum / variants.len() as f64;
            println!(
                "{}\tlitedoc\t{:.2}\t{}/{}\t{}/{}",
                case.name,
                avg,
                strict_ok,
                variants.len(),
                recovery_ok,
                variants.len()
            );
            if csv_enabled {
                csv_rows.push(format!(
                    "corpus,{},realistic,litedoc,{:.4},,,{},{},{}",
                    case.name,
                    avg,
                    strict_ok,
                    recovery_ok,
                    variants.len()
                ));
            }
            let _ = io::stdout().flush();
        }
    }
    if csv_enabled {
        let path = match csv_path {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("robustness_report.csv"),
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
    println!();
}
