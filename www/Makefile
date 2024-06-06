default: wasm-pkg

wasm-pkg:
	test -e html/pkg || ln -s ../../acb_wasm/pkg html/pkg

clean:
	test ! -e html/pkg || rm html/pkg

.PHONY: wasm-pkg clean
