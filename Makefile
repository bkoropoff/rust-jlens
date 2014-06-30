all:
	cargo build

test:
	cargo test

doc: src/jlens.rs
	rustdoc -o $@ $<

clean:
	rm -rf target doc

.PHONY: all test clean
