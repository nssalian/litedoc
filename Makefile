.PHONY: build test bench bench-quick check clean install fmt clippy doc audit build-py install-py

build:
	cargo build --release

test:
	cargo test --all

bench:
	cargo bench -p litedoc-core

bench-quick:
	cargo bench -p litedoc-core -- --quick

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

audit:
	cargo install cargo-audit --locked --force
	cargo audit

check: fmt clippy test

clean:
	cargo clean

install:
	cargo install litedoc-cli

doc:
	cargo doc --no-deps --open

# Python bindings
build-py:
	cd crates/litedoc-py && maturin build --release

install-py:
	cd crates/litedoc-py && maturin develop
