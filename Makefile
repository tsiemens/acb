build: rust

rust:
	cargo build

release:
	cargo build --release

www:
	$(MAKE) -C www

acb_wasm:
	$(MAKE) -C acb_wasm

all-notest: rust acb_wasm www

all: all-notest test

clean:
	cargo clean
	$(MAKE) -C acb_wasm clean
	$(MAKE) -C www clean

test-unit:
	# Excludes integration tests in tests/
	cargo test --lib --bins

test:
	cargo test

test-py:
	make -C py test

install:
	cargo install --path .

uninstall:
	cargo uninstall acb

.PHONY: clean test acb_wasm www