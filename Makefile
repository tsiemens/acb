export GOPATH=$(shell buildutil/find-gopath)

build:
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

clean:
	rm bld/acb
	rm bld/test.test

test:
	# -count=1 is the idiomatic way to disable test caching.
	go test ./test -v -count=1

test-bin:
	go test ./test -c -o bld/test.test
	@echo "Test binary file created: 'bld/test.test'. This should be run from the ./test/ directory"

test-py:
	make -C py test

.PHONY: clean test
