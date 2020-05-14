export GOPATH=$(shell buildutil/find-gopath)

build:
	mkdir -p bld
	go build -o bld/acb main.go

getdeps:
	go get -u github.com/spf13/cobra/cobra
	go get -u github.com/stretchr/testify
	go get -u github.com/olekukonko/tablewriter

clean:
	rm bld/acb

test:
	go test ./test -v

.PHONY: clean test
