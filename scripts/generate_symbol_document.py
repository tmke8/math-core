import re
import unicodedata as ud

CHAR_PATTERN = re.compile(r"^pub const [^:]+: \S+ =[^']+'(.)'[^;]*;", re.MULTILINE)
UNICODE_PATTERN = re.compile(
    r"^pub const [^:]+: \S+ =[^']+'\\u\{([0-9A-Fa-f]+)\}'[^;]*;", re.MULTILINE
)
DOTTED_CIRCLE = "◌"
SYMBOLS_PATH = "crates/mathml-renderer/src/symbol.rs"
OUTPUT_PATH = "scripts/all_symbols.txt"


def extract_symbols() -> str:
    with open(SYMBOLS_PATH, "r", encoding="utf-8") as f:
        content = f.read()
    symbols = CHAR_PATTERN.findall(content)
    symbols.append("\\")  # Backslash is not matched by the regex
    unicode_codes = UNICODE_PATTERN.findall(content)
    # The symbols that are specified via unicode codes are all combining diacritics.
    # Prepend a dotted circle to make them visible.
    symbols.extend(f"{DOTTED_CIRCLE}{chr(int(code, 16))}" for code in unicode_codes)
    return "".join(symbols)


def is_valid_unicode(char: str) -> bool:
    # Exclude unassigned, surrogate, and private use characters
    return ud.category(char) not in ("Cn", "Cs", "Co")


def common_unicode_blocks() -> list[str]:
    code_points: list[range] = []
    # Printable ASCII characters
    code_points.append(range(0x20, 0x7F))
    # Latin-1 Supplement
    code_points.append(range(0x80, 0x100))
    # Latin Extended-A
    code_points.append(range(0x100, 0x180))
    # Latin Extended-B
    code_points.append(range(0x180, 0x200))
    # Greek and Coptic
    code_points.append(range(0x370, 0x400))
    # Cyrillic
    code_points.append(range(0x400, 0x500))
    # Cyrillic Supplement
    code_points.append(range(0x500, 0x530))
    # Hebrew
    code_points.append(range(0x590, 0x600))
    # Arabic
    code_points.append(range(0x600, 0x700))
    # Greek Extended
    code_points.append(range(0x1F00, 0x2000))
    # General Punctuation
    code_points.append(range(0x2000, 0x2070))
    return [
        "".join(ch for cp in code_range if is_valid_unicode(ch := chr(cp)))
        for code_range in code_points
    ]


def math_script_blocks() -> list[str]:
    code_points: list[range] = []
    stragglers = ""
    # Bold script
    code_points.append(range(0x1D4D0, 0x1D504))
    # Bold italic (latin)
    code_points.append(range(0x1D468, 0x1D49C))
    # Bold italic (greek)
    code_points.append(range(0x1D71C, 0x1D756))
    # Bold (latin)
    code_points.append(range(0x1D400, 0x1D434))
    # Bold (greek)
    code_points.append(range(0x1D6A8, 0x1D6E2))
    # Bold (other)
    code_points.append(range(0x1D7CA, 0x1D7D8))
    # Fraktur
    code_points.append(range(0x1D504, 0x1D538))
    # Fraktur from other blocks
    stragglers += "ℭℌℑℜℨ"
    # Script
    code_points.append(range(0x1D49C, 0x1D4D0))
    # Script from other blocks
    stragglers += "ℬℰℱℋℐℒℳℛℯℊℴ"
    # Monospace (latin)
    code_points.append(range(0x1D670, 0x1D6A4))
    # Monospace (digits)
    code_points.append(range(0x1D7F6, 0x1D800))
    # Sans-serif (latin)
    code_points.append(range(0x1D5A0, 0x1D5D4))
    # Sans-serif (digits)
    code_points.append(range(0x1D7E2, 0x1D7EB))
    # Double-struck (latin)
    code_points.append(range(0x1D538, 0x1D56C))
    # Double-struck (digits)
    code_points.append(range(0x1D7D8, 0x1D7E2))
    # Double-struck from other blocks
    stragglers += "ℂℍℕℙℚℝℤ"
    # Italic (latin)
    code_points.append(range(0x1D434, 0x1D468))
    # Italic (greek)
    code_points.append(range(0x1D6E2, 0x1D71C))
    # Italic from other blocks
    stragglers += "ℎ𝚤𝚥"
    # Bold fraktur
    code_points.append(range(0x1D56C, 0x1D5A0))
    # Sans-serif bold italic (latin)
    code_points.append(range(0x1D63C, 0x1D670))
    # Sans-serif bold italic (greek)
    code_points.append(range(0x1D790, 0x1D7CA))
    # Sans-serif italic
    code_points.append(range(0x1D608, 0x1D63C))
    # Bold sans-serif (latin)
    code_points.append(range(0x1D5D4, 0x1D608))
    # Bold sans-serif (greek)
    code_points.append(range(0x1D756, 0x1D790))
    # Bold sans-serif (digits)
    code_points.append(range(0x1D7EC, 0x1D7F6))
    lines = [
        "".join(ch for cp in code_range if is_valid_unicode(ch := chr(cp)))
        for code_range in code_points
    ]
    lines.append(stragglers)
    return lines


def unicode_variants() -> str:
    variant0 = "\ufe00"
    variant1 = "\ufe01"
    script_chars: list[str] = []
    # Script
    script_chars.extend(
        ch for cp in range(0x1D49C, 0x1D4D0) if is_valid_unicode(ch := chr(cp))
    )
    # Script from other blocks
    script_chars += list("ℬℰℱℋℐℒℳℛℯℊℴ")
    line = "".join(f"{ch}{variant0}" for ch in script_chars)
    line += "".join(f"{ch}{variant1}" for ch in script_chars)
    line += "∅\ufe00"
    return line


def main() -> None:
    lines: list[str] = []
    lines += common_unicode_blocks()
    lines += math_script_blocks()
    lines.append(unicode_variants())
    lines.append(extract_symbols())

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))


if __name__ == "__main__":
    main()
