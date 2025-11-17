# Scripts

## Font subsetting

```sh
python scripts/generate_symbol_document.py
pip install fonttools
# results in a 210kB woff2 file
fonttools subset NewCMMath-Book-prime-roundhand-vec.otf --text-file=scripts/all_symbols.txt
# results in a 460kB woff2 file
fonttools subset NewCMMath-Book-prime-roundhand-vec.otf --text-file=scripts/all_symbols.txt --layout-features='*' --glyph-names --symbol-cmap --legacy-cmap --notdef-glyph --notdef-outline --recommended-glyphs --name-IDs='*' --name-legacy --name-languages='*'
woff2_compress NewCMMath-Book-prime-roundhand-vec.subset.otf
cp NewCMMath-Book-prime-roundhand-vec.subset.woff2 playground/NewCMMath-Book-prime-roundhand-vec.woff2
```
