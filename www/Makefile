default:
	@echo "Valid make targets: html-import, wasm, all"
	exit 1

all: html-import wasm

html-import:
	bash ./copy_system_assets.sh

wasm:
	$(MAKE) -C wasm all

.PHONY: wasm
