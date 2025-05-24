default: all

wasm-pkg:
	# Link the built wasm package to the dist directory
	test -e dist/js/pkg || \
		(mkdir -p dist/js && ln -s ../../../acb_wasm/pkg dist/js/pkg)
	# Link the built wasm package to the src directory so that the
	# linter and ts compiler can find it
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

npm-build: wasm-pkg npm-install lint
	npm run build

all: static-dist-link npm-build

clean:
	rm -r dist
	rm src/pkg

.PHONY: wasm-pkg clean
