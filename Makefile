export GOPATH=$(shell echo $$(readlink -f $$(pwd)/../../../..))

build:
	mkdir -p bld
	go build -o bld/acb main.go

getdeps:
	go get -u github.com/spf13/cobra/cobra

clean:
	rm bld/acb

test:
	echo "No tests yet..."
	# go test ./test

.PHONY: clean test
