# The Ultimate Stress Test

This is a **bold** statement with *italic* words and `inline code` mixed together.

## Nested Formatting Hell

Here's **bold with *nested italic* inside** and *italic with **nested bold** inside*.

What about ~~strikethrough with **bold** inside~~?

## Links and References

Check out [[Example Site|https://example.com]] for more info.

Or use an autolink: <https://github.com/rust-lang/rust>

Here's a footnote reference[^1] and another[^note].

## Code Blocks

```rust
fn main() {
    let message = "Hello, world!";
    println!("{}", message);
    
    // Nested backticks don't break us
    let code = "`backticks`";
}
```

```
No language specified here
Just plain code
```

## Lists

::list
- First item with **bold**
- Second item with `code`
- Third item with [[link|https://example.com]]
::

::list ordered start=5
- Fifth item
- Sixth item
- Seventh item
::

## Tables

::table
| Name | Age | City |
|------|-----|------|
| Alice | 30 | NYC |
| Bob | 25 | LA |
| Charlie | **35** | Chicago |
::

## Callouts

::callout type="warning" title="Watch Out!"
This is a warning callout.

It has **multiple** paragraphs with *formatting*.
::

::callout type="info"
Default title callout with `code` inside.
::

## Block Quotes

::quote
To be or not to be, that is the question.

Whether 'tis nobler in the mind to suffer...
::

## Math

::math display
E = mc^2

\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
::

## Figures

::figure src="diagram.png" alt="Architecture diagram" caption="Figure 1: System Architecture"
::

## Footnotes

::footnotes
[^1]: This is the first footnote with **bold**.
[^note]: This is a named footnote with `code`.
::

---

## Edge Cases

Paragraph right after thematic break.

### Empty Sections

### Back to Back Headings

#### Level 4

##### Level 5

###### Level 6

####### This should be a paragraph (7 hashes)

Text with special chars: <not-a-tag> and *unclosed italic

Final paragraph with everything: **bold**, *italic*, `code`, ~~strike~~, and [[link|url]].
