# ldcli

Command-line tool for parsing and validating LiteDoc documents.

## Install

```bash
cargo install litedoc-cli
```

## Usage

```bash
ldcli file.ld
ldcli -j file.ld
ldcli validate file.ld
ldcli stats file.ld
```

## Notes

- Use `litedoc-core` for the Rust library.
- Use `pip install litedoc-py` and `import pyld` for Python bindings.
