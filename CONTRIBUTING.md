# General

To get your environment set up, and download the source, follow the standard installation instructions (see README).

A Makefile is provided for convenience during development. A typical workflow may be:

```sh
cd <cloned repo dir>
make
make test

# Make changes to source files ...
make
make test
# Manual checks
target/debug/acb ...
```

The Makefile places the development acb binary in the ./target/debug directory inside of the repo. Once you are done and want to install into the standard path, run the standard `make install`.

# Formatting
Please run `make rustfmt` before submitting pull requests.
Some allowances are made for unit tests (mostly line-widths).