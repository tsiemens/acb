# acb Web Frontend

## Setup

### Node.js via fnm

The frontend requires **Node.js 22** (or 20.19+). The recommended way to manage
Node versions is [fnm](https://github.com/Schniz/fnm), which can be installed
via Cargo:

```sh
cargo install fnm
```

Then install the required Node version (the `.node-version` file in this
directory pins it):

```sh
fnm install   # reads .node-version, installs the correct version
```

The `scripts/npm` wrapper in this directory handles activating the correct Node
version automatically via fnm, so no shell init changes are needed. All
Makefile targets use it.

### npm dependencies

```sh
make npm-install
```
