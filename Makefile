all: cargo-build

cargo-build:
	cargo build

test: jlens
	./jlens

jlens: src/jlens.rs
	rustc --test src/jlens.rs

doc: src/jlens.rs
	rustdoc -o $@ $<

clean:
	rm -rf target jlens doc

.PHONY: all test clean cargo-build
