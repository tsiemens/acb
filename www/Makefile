default: all

wasm-pkg:
	# Link the built wasm package to the src directory so that the
	# linter, ts compiler and vite can find it
	test -e src/pkg || ln -s ../../acb_wasm/pkg src/pkg

SYMLINK_STATIC = test -e dist/$1 || ln -s ../static/$1 dist/$1

static-dist-link:
	# Symlink everything in static/ into dist/
	# eg. static/images/ -> dist/images/
	test -e dist/ || mkdir -p dist
	$(call SYMLINK_STATIC,images)
	$(call SYMLINK_STATIC,samples)

npm-install:
	npm install

lint:
	npm run lint

build:
	npm run build

all:
	npm run all

clean:
	test ! -e dist || rm -r dist
	test ! -e src/pkg || rm src/pkg

clean-all: clean
	test ! -e node_modules || rm -r node_modules

.PHONY: wasm-pkg clean
