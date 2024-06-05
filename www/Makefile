default: wasm-pkg

all: wasm-pkg

wasm-pkg:
	test -e html/pkg || ln -s ../../acb_wasm/pkg html/pkg

html-import:
	bash ./copy_system_assets.sh

go-wasm:
	$(MAKE) -C wasm all

clean:
	rm html/pkg

.PHONY: wasm-pkg go-wasm clean
