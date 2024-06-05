build: rust-wasm

rust-wasm: setup
	wasm-pack build --target web

setup:
	test -f ${HOME}/.cargo/bin/wasm-pack || \
		cargo install wasm-pack

clean:
	test ! -e pkg || rm -r pkg

.PHONY: rust-wasm setup