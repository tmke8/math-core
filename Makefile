WASM_TARGET := web
PACKAGE := math-core-wasm
PKG_DIR := playground/pkg
FLAGS := --no-typescript

WASM_FILE := $(subst -,_,$(PACKAGE)).wasm
BINDGEN_OUTPUT := $(subst -,_,$(PACKAGE))_bg.wasm

.PHONY: playground wasm bindgen optimize

playground: wasm bindgen optimize

wasm:
	cargo build --release --target wasm32-unknown-unknown --package $(PACKAGE)

bindgen: wasm
	wasm-bindgen target/wasm32-unknown-unknown/release/$(WASM_FILE) \
		--out-dir $(PKG_DIR) \
		--target $(WASM_TARGET) \
		$(FLAGS)

optimize: bindgen
	wasm-opt $(PKG_DIR)/$(BINDGEN_OUTPUT) -Os -o $(PKG_DIR)/$(BINDGEN_OUTPUT)

clean:
	rm -rf $(PKG_DIR)/*.wasm $(PKG_DIR)/*.js

equationpage:
	cargo run --example equations --package math-core > playground/equations.html

testpage:
	cargo run --example browser_test --package math-core > playground/test.html
