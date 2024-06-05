export GOPATH=$(shell buildutil/find-gopath)

build: rust

rust:
	cargo build

release:
	cargo build --release

go:
	mkdir -p bld
	go build -o bld/acb main.go

getdeps:
	go get -u github.com/spf13/cobra/cobra
	go get -u github.com/stretchr/testify
	go get -u github.com/olekukonko/tablewriter

www:
	$(MAKE) -C www all
html-import:
	$(MAKE) -C www html-import
wasm:
	$(MAKE) -C www wasm

acb_wasm:
	$(MAKE) -C acb_wasm

all-notest: rust acb_wasm www

all: all-notest test-rs

clean:
	rm bld/acb
	rm bld/test.test
	rm -r target
	$(MAKE) -C acb_wasm clean
	$(MAKE) -C www clean

test-rs-unit:
	# Excludes integration tests in tests/
	cargo test --lib --bins

test-rs:
	cargo test

test:
	# Provide -run to filter on a test name (regex)
	# -count=1 is the idiomatic way to disable test caching.
	go test ./test -v -count=1

test-bin:
	go test ./test -c -o bld/test.test
	@echo "Test binary file created: 'bld/test.test'. This should be run from the ./test/ directory"

test-py:
	make -C py test

.PHONY: clean test acb_wasm
