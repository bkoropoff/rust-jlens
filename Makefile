all: cargo-build

cargo-build:
	cargo build

test:
	cargo test

doc: src/jlens.rs
	rustdoc -o $@ $<

clean:
	rm -rf target doc

.PHONY: all test clean cargo-build
