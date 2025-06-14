# math-core

A Python library for converting LaTeX math expressions to MathML Core.

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
pip install math-core
```

## Quick Start

```python
from math_core import LatexToMathML

# Create a converter instance
converter = LatexToMathML()

# Convert inline math
mathml = converter.convert_with_local_counter("x^2 + y^2 = z^2", displaystyle=False)
print(mathml)
# Output: <math><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><msup><mi>y</mi><mn>2</mn></msup><mo>=</mo><msup><mi>z</mi><mn>2</mn></msup></math>

# Convert display math
mathml = converter.convert_with_local_counter(r"\frac{1}{2}", displaystyle=True)
print(mathml)
# Output: <math display="block"><mfrac><mn>1</mn><mn>2</mn></mfrac></math>
```

## Usage

### Basic Usage

```python
from math_core import LatexToMathML, LatexError, PrettyPrint

# Initialize converter
converter = LatexToMathML(pretty_print=PrettyPrint.ALWAYS)

# Convert LaTeX to MathML
try:
    mathml = converter.convert_with_local_counter(r"\sqrt{x^2 + 1}", displaystyle=False)
    print(mathml)
except LatexError as e:
    print(f"Conversion error: {e}")
```

### Custom LaTeX Macros

Define custom macros to extend or modify LaTeX command behavior:

```python
# Define custom macros
macros = {
    "d": r"\mathrm{d}",      # Differential d
    "R": r"\mathbb{R}",      # Real numbers
    "vec": r"\mathbf{#1}"    # Vector notation
}

converter = LatexToMathML(macros=macros)
mathml = converter.convert_with_local_counter(r"\d x", displaystyle=False)
```

### Numbered Equations with Global Counter

For documents with multiple numbered equations:

```python
converter = LatexToMathML()

# First equation gets (1)
eq1 = converter.convert_with_global_counter(
    r"\begin{align}E = mc^2\end{align}",
    displaystyle=True
)

# Second equation gets (2)
eq2 = converter.convert_with_global_counter(
    r"\begin{align}F = ma\end{align}",
    displaystyle=True
)

# Reset counter when starting a new chapter/section
converter.reset_global_counter()

# This equation gets (1) again
eq3 = converter.convert_with_global_counter(
    r"\begin{align}p = mv\end{align}",
    displaystyle=True
)
```

### Local Counter for Independent Numbering

Use local counters when equation numbers should restart within each conversion:

```python
converter = LatexToMathML()

# Each conversion has independent numbering
doc1 = converter.convert_with_local_counter(
    r"\begin{align}a &= b\\c &= d\end{align}",
    displaystyle=True
)  # Contains (1) and (2)

doc2 = converter.convert_with_local_counter(
    r"\begin{align}x &= y\\z &= w\end{align}",
    displaystyle=True
)  # Also contains (1) and (2)
```

## API Reference

### LatexToMathML

The main converter class.

**Constructor Parameters:**
- `pretty_print` (`PrettyPrint`, optional): An enum value indicating whether to pretty print the MathML output. Options are `PrettyPrint.NEVER`, `PrettyPrint.ALWAYS`, or `PrettyPrint.AUTO`. `PrettyPrint.AUTO` means that all block equations will be pretty printed. Default: `PrettyPrint.NEVER`.
- `macros` (`dict[str, str]`, optional): Dictionary of LaTeX macros for custom commands.

**Methods:**
- `convert_with_global_counter(latex: str, displaystyle: bool) -> str`: Convert LaTeX to MathML using a global equation counter.
- `convert_with_local_counter(latex: str, displaystyle: bool) -> str`: Convert LaTeX to MathML using a local equation counter.
- `reset_global_counter() -> None`: Reset the global equation counter to zero.

### LatexError

Exception raised when LaTeX parsing or conversion fails.

```python
from math_core import LatexError

try:
    result = converter.convert_with_local_counter(r"\invalid", displaystyle=False)
except LatexError as e:
    print(f"Invalid LaTeX: {e}")
```

## Use Cases

### Static Site Generators

Integrate `math-core` into your static site generator to convert LaTeX in Markdown files:

```python
import re
from math_core import LatexToMathML, PrettyPrint

converter = LatexToMathML(pretty_print=PrettyPrint.AUTO)

def process_math(content):
    # Replace display math $$...$$; do this first to avoid conflicts with inline math delimiters
    content = re.sub(
        r"\$\$([^\$]+)\$\$",
        lambda m: converter.convert_with_local_counter(m.group(1), displaystyle=True),
        content,
    )

    # Replace inline math $...$
    content = re.sub(
        r"\$([^\$]+)\$",
        lambda m: converter.convert_with_local_counter(m.group(1), displaystyle=False),
        content,
    )

    return content
```

### Web Applications

Generate MathML on the server side:

```python
from flask import Flask, render_template_string
from math_core import LatexToMathML, LatexError

app = Flask(__name__)
converter = LatexToMathML()

@app.route("/equation/<latex>")
def render_equation(latex):
    try:
        mathml = converter.convert_with_local_counter(latex, displaystyle=True)
        return render_template_string(
            "<html><body>{{ mathml|safe }}</body></html>", mathml=mathml
        )
    except LatexError:
        return "Invalid equation", 400
```

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
