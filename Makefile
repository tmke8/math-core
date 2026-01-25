WASM_TARGET := wasm32-unknown-unknown
PACKAGE := math-core-wasm
PKG_DIR := playground/pkg
WASM_FILE := math_core_wasm_bg.wasm

.PHONY: build wasm bindgen optimize

build: wasm bindgen optimize

wasm:
	cargo build --release --target $(WASM_TARGET) --package $(PACKAGE)

bindgen: wasm
	wasm-bindgen target/$(WASM_TARGET)/release/math_core_wasm.wasm \
		--out-dir $(PKG_DIR) \
		--target web \
		--no-typescript

optimize: bindgen
	wasm-opt $(PKG_DIR)/$(WASM_FILE) -Os -o $(PKG_DIR)/$(WASM_FILE)

clean:
	rm -rf $(PKG_DIR)/*.wasm $(PKG_DIR)/*.js
