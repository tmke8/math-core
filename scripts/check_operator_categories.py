"""
Compare operator categories in symbol.rs against the MathML Core spec.

This script parses the operator dictionary from the W3C MathML Core spec
(operator-dictionary-compact.html) and compares it against the categories
assigned to Unicode characters in crates/mathml-renderer/src/symbol.rs.

Usage:
    python3 scripts/check_operator_categories.py

The spec file is fetched from:
    https://raw.githubusercontent.com/w3c/mathml-core/refs/heads/main/tables/operator-dictionary-compact.html
"""

import re
import sys
import urllib.request

SPEC_URL = "https://raw.githubusercontent.com/w3c/mathml-core/refs/heads/main/tables/operator-dictionary-compact.html"
SYMBOLS_PATH = "crates/mathml-renderer/src/symbol.rs"


def fetch_spec():
    """Fetch the operator dictionary compact HTML from the spec."""
    with urllib.request.urlopen(SPEC_URL) as resp:
        return resp.read().decode("utf-8")


def parse_ranges(text):
    """Parse Unicode ranges like [U+2190–U+2195] and {U+002B} into a set of codepoints."""
    codepoints = set()
    for m in re.finditer(r"\[U\+([0-9A-F]+)[–-]U\+([0-9A-F]+)\]", text):
        start, end = int(m.group(1), 16), int(m.group(2), 16)
        for cp in range(start, end + 1):
            codepoints.add(cp)
    for m in re.finditer(r"(?<!\[)U\+([0-9A-F]+)(?!\s*[–-])", text):
        codepoints.add(int(m.group(1), 16))
    return codepoints


def parse_spec_categories(html):
    """Extract category mappings from the spec HTML."""
    spec = {}
    # Match rows like: <td>... entries ... in <strong>form</strong> form: <code>ranges</code></td><td>X</td>
    for m in re.finditer(
        r"<td>.*?<strong>(\w+)</strong> form: <code>([^<]+)</code></td><td>(\w)</td>",
        html,
    ):
        form, ranges_text, category = m.group(1), m.group(2), m.group(3)
        for cp in parse_ranges(ranges_text):
            if cp not in spec:
                spec[cp] = set()
            spec[cp].add(category)

    # Also match rows without <strong> (single entries like category F/G)
    for m in re.finditer(
        r"<td>(\d+) entries in <strong>(\w+)</strong> form: <code>([^<]+)</code></td><td>(\w)</td>",
        html,
    ):
        form, ranges_text, category = m.group(2), m.group(3), m.group(4)
        for cp in parse_ranges(ranges_text):
            if cp not in spec:
                spec[cp] = set()
            spec[cp].add(category)

    return spec


def parse_symbol_rs(path):
    """Parse symbol.rs and extract all active (non-commented) symbol definitions."""
    with open(path) as f:
        content = f.read()

    symbols = []
    for line in content.split("\n"):
        stripped = line.strip()
        if stripped.startswith("//"):
            continue

        # Match Rel::new('X', RelCategory::Y)
        m = re.search(
            r"pub const (\w+):\s*Rel\s*=\s*Rel::new\('(\\u\{[0-9A-Fa-f]+\}|.)'\s*,\s*RelCategory::(\w+)\)",
            line,
        )
        if m:
            symbols.append((m.group(1), _parse_char(m.group(2)), "Rel", m.group(3)))
            continue

        # Match Bin::new('X', BinCategory::Y)
        m = re.search(
            r"pub const (\w+):\s*Bin\s*=\s*Bin::new\('(\\u\{[0-9A-Fa-f]+\}|.)'\s*,\s*BinCategory::(\w+)\)",
            line,
        )
        if m:
            symbols.append((m.group(1), _parse_char(m.group(2)), "Bin", m.group(3)))
            continue

        # Match Op::new('X', OpCategory::Y)
        m = re.search(
            r"pub const (\w+):\s*Op\s*=\s*Op::new\('(\\u\{[0-9A-Fa-f]+\}|.)'\s*,\s*OpCategory::(\w+)\)",
            line,
        )
        if m:
            symbols.append((m.group(1), _parse_char(m.group(2)), "Op", m.group(3)))
            continue

        # Match OrdLike::new('X', OrdCategory::Y)
        m = re.search(
            r"pub const (\w+):\s*OrdLike\s*=\s*OrdLike::new\('(\\u\{[0-9A-Fa-f]+\}|.)'\s*,\s*OrdCategory::(\w+)\)",
            line,
        )
        if m:
            symbols.append(
                (m.group(1), _parse_char(m.group(2)), "OrdLike", m.group(3))
            )
            continue

    return symbols


def _parse_char(s):
    """Parse a character literal from Rust source, returning its codepoint."""
    if s.startswith("\\u{"):
        return int(s[3:-1], 16)
    return ord(s)


def compare(spec, symbols):
    """Compare symbol.rs categories against the spec and report mismatches."""
    mismatches = []

    for name, cp, typ, cat in symbols:
        spec_cats = spec.get(cp, set())
        char_repr = chr(cp)
        cp_str = f"U+{cp:04X}"

        if typ == "Rel":
            if cat == "Default" and "A" in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"Rel::Default", "Rel::A (spec: category A, stretchy)")
                )
            elif cat == "A" and "A" not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"Rel::A", "Rel::Default (spec: not category A)")
                )

        elif typ == "Bin":
            if "C" in spec_cats and "B" not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"Bin::{cat}", f"Op::C (spec: category C)")
                )
            elif "B" not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"Bin::{cat}", f"not B (spec: {spec_cats})")
                )
            elif cat == "BD" and "D" not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, "Bin::BD", "Bin::B (spec: B only, no prefix D form)")
                )
            elif cat == "B" and "D" in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, "Bin::B", "Bin::BD (spec: also has prefix D form)")
                )

        elif typ == "Op":
            if cat not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"Op::{cat}", f"spec: {spec_cats}")
                )

        elif typ == "OrdLike":
            expected = cat
            if cat == "KButUsedToBeB":
                expected = "K"
            elif cat == "FGandForceDefault":
                if "F" not in spec_cats or "G" not in spec_cats:
                    mismatches.append(
                        (name, char_repr, cp_str, f"OrdLike::{cat}", f"spec: {spec_cats}")
                    )
                continue

            if expected and expected not in spec_cats:
                mismatches.append(
                    (name, char_repr, cp_str, f"OrdLike::{cat}", f"spec: {spec_cats}")
                )

    return mismatches


def main():
    print("Fetching MathML Core operator dictionary...")
    html = fetch_spec()
    spec = parse_spec_categories(html)
    print(f"Parsed {sum(len(v) for v in spec.values())} category entries from spec.")

    symbols = parse_symbol_rs(SYMBOLS_PATH)
    print(f"Parsed {len(symbols)} symbol definitions from symbol.rs.")

    mismatches = compare(spec, symbols)

    if not mismatches:
        print("\nAll categories match the spec!")
        return 0

    print(f"\n{len(mismatches)} mismatch(es) found:\n")
    for name, char_repr, cp_str, current, expected in mismatches:
        print(f"  {name} ({char_repr}, {cp_str})")
        print(f"    current:  {current}")
        print(f"    expected: {expected}")
        print()

    return 1


if __name__ == "__main__":
    sys.exit(main())
