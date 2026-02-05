@profile litedoc
@modules tables, footnotes, math

--- meta ---
title: "LiteDoc Test Document"
author: "Test Author"
version: 1
draft: false
tags: [docs, test, demo]
---

# Welcome to LiteDoc

This is a paragraph with *emphasis*, **strong text**, and `inline code`.
It also has a [[link|https://example.com]] and a footnote reference.[^1]

## Lists

::list unordered
- First item
| This is a continuation of the first item
| with multiple lines
- Second item
- Third item
::

::list ordered start=1
- Step one
- Step two
- Step three
::

## Code Blocks

```rust
fn main() {
    println!("Hello, LiteDoc!");
}
```

```python
def greet(name):
    return f"Hello, {name}!"
```

## Callouts

::callout type=note title="Important Note"
This is a callout block. It draws attention to important information.

You can have multiple paragraphs inside.
::

::callout type=warning title="Warning"
Be careful with this feature!
::

## Quotes

::quote
The best way to predict the future is to invent it.

-- Alan Kay
::

## Tables

::table
| Name     | Language | Year |
| -------- | -------- | ---- |
| Rust     | Systems  | 2010 |
| Python   | General  | 1991 |
| LiteDoc  | Markup   | 2024 |
::

## Math

Inline math: \( E = mc^2 \)

::math block
\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}
::

## Figures

::figure src="diagram.png" alt="Architecture diagram" caption="System Overview"
::

## Footnotes

::footnotes
[^1]: This is the footnote content with *emphasis*.
::

---

That's all folks! LiteDoc is **token-efficient** and *human-readable*.
