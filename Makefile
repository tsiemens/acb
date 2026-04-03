SHELL := /bin/bash

build: rust

ensure-fnm:
	@if ! command -v fnm &>/dev/null && ! test -f "$$HOME/.cargo/bin/fnm"; then \
		echo ""; \
		echo "WARNING: fnm (Fast Node Manager) is not installed."; \
		echo "Without it, the pdf-text Node.js (pdfjs-dist) reader and the webapp will not work."; \
		echo ""; \
		read -p "Install fnm via 'cargo install fnm'? [y/N] " answer; \
		if [ "$$answer" = "y" ] || [ "$$answer" = "Y" ]; then \
			cargo install fnm; \
		else \
			echo "Skipping fnm install. Node.js features will be unavailable."; \
		fi; \
	fi

rust: ensure-fnm
	cargo build

release:
	cargo build --release

www:
	$(MAKE) -C www build

www-all:
	$(MAKE) -C www all

acb_wasm:
	$(MAKE) -C acb_wasm

wasm: acb_wasm

web: acb_wasm www

web-all: acb_wasm www-all

all-rust-notest: rust acb_wasm

all-notest: all-rust-notest web-all

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

rustfmt:
	rustfmt --config-path . `find src tests acb_wasm www -type f -name '*.rs'`

check-rustfmt:
	rustfmt --check --config-path . `find src tests acb_wasm www -type f -name '*.rs'`

install:
	cargo install --path .

uninstall:
	cargo uninstall acb

.PHONY: clean test acb_wasm www www-all
