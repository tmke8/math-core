WASM_TARGET := web
PACKAGE := math-core-wasm
PKG_DIR := playground/pkg
FLAGS := --no-typescript

WASM_FILE := $(subst -,_,$(PACKAGE)).wasm
BINDGEN_OUTPUT := $(subst -,_,$(PACKAGE))_bg.wasm

SUBSET_OTF := playground/fonts/NewCMMath-Book-prime-roundhand-vec-subset.otf
SUBSET_WOFF2 := playground/fonts/NewCMMath-Book-prime-roundhand-vec-subset.woff2
SOURCE_OTF := playground/fonts/NewCMMath-Book-prime-roundhand-vec.otf

.PHONY: playground wasm bindgen optimize clean equationpage testpage comparison subset allsymbols

playground: wasm bindgen optimize playground/mathmlfixes.css $(SUBSET_WOFF2)

playground/mathmlfixes.css: css/mathmlfixes.css
	cp $< $@

wasm:
	cargo build --release --target wasm32-unknown-unknown --package $(PACKAGE)

bindgen: wasm
	wasm-bindgen target/wasm32-unknown-unknown/release/$(WASM_FILE) \
		--out-dir $(PKG_DIR) \
		--target $(WASM_TARGET) \
		$(FLAGS)

optimize: bindgen
	wasm-opt $(PKG_DIR)/$(BINDGEN_OUTPUT) -Os -o $(PKG_DIR)/$(BINDGEN_OUTPUT).tmp
	mv $(PKG_DIR)/$(BINDGEN_OUTPUT).tmp $(PKG_DIR)/$(BINDGEN_OUTPUT)

clean:
	rm -rf $(PKG_DIR)/*.wasm $(PKG_DIR)/*.js

equationpage: $(SUBSET_WOFF2)
	cargo run --example equations --package math-core > playground/equations.html

testpage: $(SUBSET_WOFF2)
	cargo run --example browser_test --package math-core > playground/test.html

comparison: $(SUBSET_WOFF2)
	cargo run --bin mathcore -- -c docs/mathcore.toml --inline-del ₮ --block-del ₮₮ --continue-on-error - < docs/comparison.html > playground/comparison.html

allsymbols: scripts/all_symbols.txt

scripts/all_symbols.txt: crates/mathml-renderer/src/symbol.rs playground/index.html
	python3 scripts/generate_symbol_document.py

subset: $(SUBSET_WOFF2)

$(SUBSET_OTF): scripts/all_symbols.txt $(SOURCE_OTF)
	hb-subset \
		--output-file=$@ \
		--text-file=$< \
		--layout-features=ssty,kern,aalt \
		--desubroutinize \
		$(SOURCE_OTF)

$(SUBSET_WOFF2): $(SUBSET_OTF)
	woff2_compress $(SUBSET_OTF)
