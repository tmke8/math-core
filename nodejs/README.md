# math-core

A Node.js library for converting LaTeX math expressions to MathML Core.

## Overview

`math-core` converts LaTeX mathematical expressions into MathML Core, a streamlined subset of MathML that is supported by all major web browsers. It lets you render mathematical content on the web without requiring JavaScript libraries or polyfills.

## Features

- Convert LaTeX math expressions to MathML Core
- Support for both inline and display (block) math
- Define custom LaTeX macros for extended functionality
- Global and local counter for numbered equations
- Pretty-printing option for readable MathML output
- Comprehensive error handling with descriptive error messages

## Installation

```bash
npm i math-core
```

> **Note:** This package is for Node.js (and compatible runtimes like Bun) only. It cannot be used in the browser, because it uses `readFileSync` from `node:fs` to load the underlying WASM module.

## Quick Start

```javascript
import { LatexToMathML } from "math-core";

const converter = new LatexToMathML({});

// Convert inline math
const inline = converter.convert_with_local_counter("x^2 + y^2 = z^2", false);
console.log(inline);
// Output: <math><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><msup><mi>y</mi><mn>2</mn></msup><mo>=</mo><msup><mi>z</mi><mn>2</mn></msup></math>

// Convert display math
const display = converter.convert_with_local_counter("\\frac{1}{2}", true);
console.log(display);
// Output: <math display="block"><mfrac><mn>1</mn><mn>2</mn></mfrac></math>
```

## Usage

### Basic Usage

```javascript
import { LatexToMathML } from "math-core";

const converter = new LatexToMathML({ prettyPrint: "always" });

try {
  const mathml = converter.convert_with_local_counter("\\sqrt{x^2 + 1}", false);
  console.log(mathml);
} catch (e) {
  console.error(`Conversion error: ${e.message}`);
}
```

### Custom LaTeX Macros

Define custom macros to extend or modify LaTeX command behavior:

```javascript
const macros = new Map();
macros.set("d", "\\mathrm{d}");    // Differential d
macros.set("R", "\\mathbb{R}");    // Real numbers
macros.set("vec", "\\mathbf{#1}"); // Vector notation

const converter = new LatexToMathML({ macros });
const mathml = converter.convert_with_local_counter("\\d x", false);
```

### Numbered Equations with Global Counter

For documents with multiple numbered equations:

```javascript
const converter = new LatexToMathML({});

// First equation gets (1)
const eq1 = converter.convert_with_global_counter(
  "\\begin{align}E = mc^2\\end{align}",
  true,
);

// Second equation gets (2)
const eq2 = converter.convert_with_global_counter(
  "\\begin{align}F = ma\\end{align}",
  true,
);

// Reset counter when starting a new chapter/section
converter.reset_global_counter();

// This equation gets (1) again
const eq3 = converter.convert_with_global_counter(
  "\\begin{align}p = mv\\end{align}",
  true,
);
```

### Local Counter for Independent Numbering

Use local counters when equation numbers should restart within each conversion:

```javascript
const converter = new LatexToMathML({});

// Each conversion has independent numbering
const doc1 = converter.convert_with_local_counter(
  "\\begin{align}a &= b\\\\c &= d\\end{align}",
  true,
); // Contains (1) and (2)

const doc2 = converter.convert_with_local_counter(
  "\\begin{align}x &= y\\\\z &= w\\end{align}",
  true,
); // Also contains (1) and (2)
```

### Error Handling

By default, conversion errors throw a `LatexError` with detailed diagnostics:

```javascript
const converter = new LatexToMathML({});

try {
  converter.convert_with_local_counter("\\begin{foobar}", false);
} catch (e) {
  console.log(e.message); // 'Unknown environment "foobar".'
  console.log(e.report);  // Formatted diagnostic with source spans
  console.log(e.context); // The relevant LaTeX source
  console.log(e.start);   // Start offset of the error
  console.log(e.end);     // End offset of the error
}
```

Set `throwOnError: false` to return an HTML error snippet instead of throwing:

```javascript
const converter = new LatexToMathML({ throwOnError: false });
const result = converter.convert_with_local_counter("\\invalid", false);
// Returns: <span class="math-core-error" title="..."><code>\invalid</code></span>
```

## API Reference

### `LatexToMathML`

The main converter class.

**Constructor:**

```typescript
new LatexToMathML(options: MathCoreOptions)
```

**Options:**

| Option | Type | Default | Description |
|---|---|---|---|
| `prettyPrint` | `"never" \| "always" \| "auto"` | `"never"` | Whether to pretty-print the MathML output. `"auto"` pretty-prints block equations only. |
| `macros` | `Map<string, string>` | — | Custom LaTeX macros. |
| `xmlNamespace` | `boolean` | `false` | Include `xmlns="http://www.w3.org/1998/Math/MathML"` in the `<math>` tag. |
| `throwOnError` | `boolean` | `true` | Throw `LatexError` on conversion errors. If `false`, returns an HTML error snippet instead. |
| `ignoreUnknownCommands` | `boolean` | `false` | Render unknown commands as red text instead of erroring. |
| `annotation` | `boolean` | `false` | Include the original LaTeX as an annotation in the MathML output. |

**Methods:**

- `convert_with_global_counter(latex: string, displaystyle: boolean): string` — Convert LaTeX to MathML using a global equation counter.
- `convert_with_local_counter(latex: string, displaystyle: boolean): string` — Convert LaTeX to MathML using a local equation counter.
- `reset_global_counter(): void` — Reset the global equation counter to zero.

### `LatexError`

Error thrown when LaTeX parsing or conversion fails.

**Properties:**

| Property | Type | Description |
|---|---|---|
| `message` | `string` | Description of the error. |
| `report` | `string \| undefined` | Formatted diagnostic report with source spans. |
| `context` | `string \| undefined` | The relevant LaTeX source. |
| `start` | `number` | Start offset of the error in the source. |
| `end` | `number` | End offset of the error in the source. |

## Why MathML Core?

MathML Core is a carefully selected subset of MathML 4 that focuses on essential mathematical notation while ensuring consistent rendering across browsers. Unlike full MathML or JavaScript-based solutions:

- **Native browser support**: No JavaScript required
- **Accessibility**: Better screen reader support
- **Performance**: Faster rendering than JS solutions
- **SEO-friendly**: Search engines can index mathematical content
- **Future-proof**: Part of web standards with ongoing browser support

## Browser Support

Firefox currently has the most complete support for MathML Core, with Chrome close behind. Safari has the least support and some rendering issues exist when using MathML Core, but it is improving with each release.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
