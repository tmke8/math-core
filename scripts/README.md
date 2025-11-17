# Scripts

## Font subsetting

```sh
python scripts/generate_symbol_document.py
pip install fonttools
fonttools subset NewCMMath-Book-prime-roundhand-vec.otf --text-file=scripts/all_symbols.txt
woff2_compress NewCMMath-Book-prime-roundhand-vec.subset.otf
cp NewCMMath-Book-prime-roundhand-vec.subset.woff2 playground/NewCMMath-Book-prime-roundhand-vec.woff2
```
