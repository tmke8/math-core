# Math Core WASM

Build with

```sh
cargo build --release --target wasm32-unknown-unknown --package math-core-wasm
```

## Features

- `all-conversions` (enabled by default): adds `convert_with_global_state` and `convert_all` to
  `LatexToMathML`.
- `interactive` (enabled by default): adds the `InteractiveDocument` class, for documents
  whose math changes over time, as in an editor. It keeps all math snippets of a document,
  and when some of them are edited, it reports which snippets have a different output now
  (because of a new equation number, a changed `\gdef`, …). `make interactive` builds a
  playground for it in `playground/interactive/`.
