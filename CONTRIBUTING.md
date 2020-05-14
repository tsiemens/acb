To get your environment set up, and download the source, follow the standard installation instructions (see README).

A Makefile is provided for convenience during development. A typical workflow may be:

```sh
cd $GOPATH/src/github.com/tsiemens/acb
make getdeps
make
make test

# Make changes to source files ...
make
make test
# Manual checks
bld/acb ...
```

The Makefile places the development acb binary in the ./bld/ directory inside of the repo. Once you are done and want to install into the standard path, run the standard ``go install` command.

Code must comply with `go fmt`