NPM = scripts/npm

default: all

wasm-pkg:
	# Link the built wasm package to the src directory so that the
	# linter, ts compiler and vite can find it
	test -e src/pkg || ln -s ../../acb_wasm/pkg src/pkg

SYMLINK_STATIC = test -e dist/$1 || ln -s ../static/$1 dist/$1

# Sets up the production portion of dist/: images and root-level sample files.
# dist/samples/ is a real directory (not a symlink) so that dist/samples/debug/
# can live alongside the production files.
# Activated via the npm scripts in package.json (see "dist-static-links-and-debug" script).
static-dist-link:
	test -e dist/ || mkdir -p dist
	$(call SYMLINK_STATIC,images)
	test -e dist/samples || mkdir -p dist/samples
	for f in static/samples/*; do test -e dist/samples/$$(basename $$f) || ln -s ../../$$f dist/samples/$$(basename $$f); done

# Adds the debug layer on top of static-dist-link: symlinks tests/data as
# dist/samples/debug/ and generates the sample manifest.
debug-samples: static-dist-link
	test -e dist/samples/debug || ln -s ../../../tests/data dist/samples/debug
	python3 scripts/gen_debug_samples_manifest.py

# Entry point for npm scripts (see package.json "dist-static-links-and-debug").
# Runs both production dist setup and debug samples.
dist-static-links-and-debug: static-dist-link debug-samples

npm-install:
	$(NPM) install

lint:
	$(NPM) run lint

build:
	$(NPM) run build

all:
	$(NPM) run all

clean:
	test ! -e dist || rm -r dist
	test ! -e src/pkg || rm src/pkg

clean-all: clean
	test ! -e node_modules || rm -r node_modules

.PHONY: wasm-pkg static-dist-link debug-samples dist-static-links-and-debug clean
