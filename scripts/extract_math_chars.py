#!/usr/bin/env python3
"""Collect the characters used inside MathML <math> elements in HTML files.

The output is a plain text file, one copy of each distinct code point, meant to be fed
to

    hb-subset --text-file=OUTPUT ...

so that a math font can be subset down to the code points actually used. Without
--output the characters are written to standard output; the progress and warning
messages always go to standard error, so the stream stays clean for a pipe.

Python 3.9+, standard library only.
"""

from collections.abc import Generator, Iterable
from typing import Final, Optional

import argparse
import os
import re
import sys
import unicodedata
from html.parser import HTMLParser
from pathlib import Path

HTML_SUFFIXES = {".html", ".htm", ".xhtml"}

# Never contribute characters: these hold the TeX/Content-MathML source.
ALWAYS_SKIP_TAGS = {"annotation", "annotation-xml"}

# Skipped only with --skip-text.
TEXT_TAGS = {"mtext"}

# RAWTEXT elements: a browser reads their content as literal text, so any markup inside
# them is never rendered as math. HTMLParser only knew about "script" and "style" until
# bpo/gh-137836; adding these keeps the result the same on interpreters that predate
# that fix. ("plaintext" needs the parser's own special casing and cannot be handled
# here, but it has been obsolete for decades.)
RAWTEXT_EXTRA = {"xmp", "iframe", "noembed", "noframes"}

LEFTOVER_ENTITY_RE = re.compile(r"&#?[0-9A-Za-z][0-9A-Za-z]*;")

# Characters a renderer draws for an element although they appear nowhere in its
# content. The radical of <msqrt>/<mroot> is the case that matters: the markup names
# the operation, not the symbol, so U+221A never reaches handle_data() and would be
# missing from the subset. Its size variants and assembly parts are reached through the
# MATH table from that one glyph, so the single code point is enough.
#
# <mfenced> is deliberately absent. It would imply "(", ")" and ",", but it was dropped
# from MathML Core and current browsers do not render it, so adding those would inflate
# the subset for markup that produces nothing. Add it here if the input predates Core
# and is read by an engine that still supports it.
IMPLIED_CHARS: Final[dict[str, str]] = {
    "msqrt": "\u221a",
    "mroot": "\u221a",
}


def _build_italic_map() -> dict[str, str]:
    """MathML Core appendix C.1, the table behind text-transform: math-auto.

    Built rather than spelled out: it is a few contiguous runs plus the holes where
    Unicode had already encoded a letterlike symbol. Verified entry for entry against
    the appendix; the 112 pairs agree exactly.
    """
    m = {chr(0x41 + i): chr(0x1D434 + i) for i in range(26)}
    m.update({chr(0x61 + i): chr(0x1D44E + i) for i in range(26)})
    # U+1D455 is unassigned; italic h had been encoded as the Planck constant.
    m["h"] = "\u210e"
    # Uppercase Greek. The unassigned source slot U+03A2 lines up with CAPITAL THETA
    # SYMBOL in the target run, so the offset holds either side of it.
    m.update(
        {chr(c): chr(0x1D6E2 + c - 0x391) for c in range(0x391, 0x3AA) if c != 0x3A2}
    )
    m.update({chr(c): chr(0x1D6FC + c - 0x3B1) for c in range(0x3B1, 0x3CA)})
    m.update(
        {
            "\u03f4": "\U0001d6f3",  # capital theta symbol
            "\u2207": "\U0001d6fb",  # nabla
            "\u2202": "\U0001d715",  # partial differential
            "\u03f5": "\U0001d716",  # epsilon symbol
            "\u03d1": "\U0001d717",  # theta symbol
            "\u03f0": "\U0001d718",  # kappa symbol
            "\u03d5": "\U0001d719",  # phi symbol
            "\u03f1": "\U0001d71a",  # rho symbol
            "\u03d6": "\U0001d71b",  # pi symbol
            "\u0131": "\U0001d6a4",  # dotless i
            "\u0237": "\U0001d6a5",  # dotless j
        }
    )
    return m


ITALIC_MAP: Final[dict[str, str]] = _build_italic_map()

# Only "normal" is a no-op: it suppresses the transform and leaves the character as
# written, which handle_data() has already collected. Every other value substitutes
# different characters; "italic" is applied below and the rest are reported so they
# cannot fail silently.
NOOP_VARIANTS = {"normal"}

# C0 controls and DEL. No font carries glyphs for these, so they would only pad the
# hb-subset input. Includes also tab, newline and carriage return.
CONTROL_CHARS = frozenset(map(chr, list(range(0x20)) + [0x7F]))


def local_name(tag: str) -> str:
    """Strip a namespace prefix: 'm:math' -> 'math'."""
    return tag.rpartition(":")[2].lower()


class CharCollector:
    """Accumulates the distinct characters of a stream of text chunks.

    Text nodes inside math markup repeat heavily ("x", "=", "\u2211", the indentation
    between elements), so each distinct chunk is remembered and a repeat costs one
    C-level set lookup instead of a walk over its characters. On repetitive input that
    is roughly an order of magnitude faster than folding every chunk into the character
    set; on input with no repeats it costs one extra hash per chunk.

    The memo is bounded, so pathological input cannot blow up memory: past the limits it
    simply stops helping.
    """

    MEMO_MAX_ENTRIES: Final = 200000
    MEMO_MAX_LEN: Final = 1024

    def __init__(self) -> None:
        self._seen: set[str] = set()
        self._done: set[str] = set()
        self.total = 0
        self.leftovers: set[str] = set()
        self.variants: set[str] = set()

    def add(self, text: str) -> None:
        self.total += len(text)
        if text in self._done:
            return
        if len(text) <= self.MEMO_MAX_LEN and len(self._done) < self.MEMO_MAX_ENTRIES:
            self._done.add(text)
        if "&" in text:
            # Named references outside the HTML5 set survive unresolved; they have to be
            # spotted here, before deduplication scrambles them.
            self.leftovers.update(LEFTOVER_ENTITY_RE.findall(text))
        self._seen.update(text)

    def text(self) -> str:
        """The distinct characters, in code point order, controls dropped.

        The set is closed under canonical decomposition: for every character its NFD
        form is added as well. hb-subset does no normalization of its own, so a
        precomposed character the font has no glyph for is simply dropped -- while the
        shaper, which does normalize, would have rendered it from its base and marks.
        Without the closure those are missing from the subset too and the result is a
        notdef box. Math fonts often cover precomposed Latin only patchily, so this is
        worth the handful of extra glyphs.

        The closure runs in one direction only. Composing "e" + U+0300 back into U+00E8
        would need the two to be adjacent, and adjacency is deliberately not tracked
        here. That direction is safe anyway: finding no precomposed glyph, the shaper
        keeps the sequence decomposed and positions the mark with GPOS.
        """
        chars = set(self._seen)
        for ch in self._seen:
            nfd = unicodedata.normalize("NFD", ch)
            if len(nfd) > 1 or nfd != ch:  # decomposable, or a singleton like U+2126
                chars.update(nfd)
        return "".join(sorted(chars - CONTROL_CHARS))


class MathExtractor(HTMLParser):
    """Feeds the text nodes inside <math> elements into a CharCollector.

    Nesting is tracked with a stack of open element names rather than a depth counter,
    because HTMLParser is a tokenizer: it reports tags as it meets them and never closes
    anything implicitly. An end tag pops through to the innermost matching open element,
    the way a browser recovers, so an element left unclosed inside math cannot strand
    the parser in the wrong state -- whether it is a void element written bare (<br>),
    sloppy markup (<mtext><b>x</mtext>), or a stray end tag from elsewhere on the page.

    The stack only ever starts at a <math> start tag, so a non-empty stack means "inside
    math" and the rest of the document costs one test per tag.
    """

    def __init__(self, sink: CharCollector, skip_text: bool = False):
        # convert_charrefs=True resolves &alpha;, &#x2211;, &InvisibleTimes; etc. to
        # real Unicode characters before handle_data() sees them.
        super().__init__(convert_charrefs=True)
        # Union rather than assignment, so a newer interpreter that knows about further
        # RAWTEXT elements keeps them. Note that "noscript" is deliberately left out:
        # with scripting disabled a browser renders its content, so math in there really
        # is needed.
        self.CDATA_CONTENT_ELEMENTS = tuple(  # pyright: ignore
            set(self.CDATA_CONTENT_ELEMENTS) | RAWTEXT_EXTRA
        )
        self.sink = sink
        self.skip_tags = (
            (ALWAYS_SKIP_TAGS | TEXT_TAGS) if skip_text else ALWAYS_SKIP_TAGS
        )
        self._stack: list[str] = []
        self._skip_open = 0
        # Content of the <mi> currently open, if any. It has to be buffered rather than
        # tested as it arrives: the transform depends on the length of the whole token,
        # and HTMLParser can report one text node in several chunks.
        self._mi: Optional[list[str]] = None

    # <foo/> falls back to handle_starttag + handle_endtag, which pushes and immediately
    # pops.

    def handle_starttag(self, tag: str, attrs: list[tuple[str, Optional[str]]]) -> None:
        name = local_name(tag)
        if not self._stack:
            if name != "math":
                return  # outside math; nothing to track
        elif name in self.skip_tags:
            self._skip_open += 1
        self._stack.append(name)
        implied = IMPLIED_CHARS.get(name)
        # Suppressed inside a skipped subtree, exactly like the text there: an <msqrt>
        # in <annotation-xml> is source, not something the page draws.
        if implied is not None and not self._skip_open:
            self.sink.add(implied)
        if name == "mi" and not self._skip_open:
            self._flush_mi()  # an unclosed previous one
            variant = None
            for key, value in attrs:
                if local_name(key) == "mathvariant" and value is not None:
                    variant = value.strip().lower()
            self._start_mi(variant)

    def handle_endtag(self, tag: str) -> None:
        if not self._stack:
            return
        name = local_name(tag)
        for i in range(len(self._stack) - 1, -1, -1):
            if self._stack[i] == name:
                for closed in self._stack[i:]:
                    if closed in self.skip_tags:
                        self._skip_open -= 1
                    elif closed == "mi":
                        self._flush_mi()
                del self._stack[i:]
                return
        # No matching open element: a stray end tag. Ignore it.

    def _start_mi(self, variant: Optional[str]) -> None:
        """Begin collecting an <mi>, unless mathvariant rules the transform out."""
        if variant is None:
            # The default. MathML Core's UA stylesheet gives <mi> text-transform:
            # math-auto, which italicises a token of exactly one character and leaves
            # anything longer -- "sin", "max" -- upright.
            self._mi = []
            return
        self._mi = None
        # "normal" is the only value Core keeps, and it suppresses the transform: the
        # character as written is what gets drawn, and handle_data() already has it.
        # Anything else is outside Core, so its substitutions are not applied here and
        # the omission is reported rather than left silent.
        if variant not in NOOP_VARIANTS:
            self.sink.variants.add(variant)

    def _flush_mi(self) -> None:
        """Apply the italic transform to the buffered <mi> token."""
        if self._mi is None:
            return
        # The chunks are joined because HTMLParser can report one text node in several
        # pieces. The strip is a deliberate over-approximation: Core applies the
        # transform to a text node of exactly one character and says nothing about
        # trimming, so "<mi> x </mi>" may or may not be italicised depending on the
        # engine. Assuming it is costs one glyph; assuming it is not risks a notdef.
        text, self._mi = "".join(self._mi).strip(), None
        if len(text) == 1:
            italic = ITALIC_MAP.get(text)
            if italic is not None:
                self.sink.add(italic)

    def close(self) -> None:
        super().close()
        self._flush_mi()  # an <mi> left open at end of file

    def handle_data(self, data: str) -> None:
        if self._stack and not self._skip_open:
            self.sink.add(data)
            if self._mi is not None:
                self._mi.append(data)


def iter_html_files(root: Path, recursive: bool) -> Generator[Path, None, None]:
    """Yield the HTML files to process, in a deterministic order."""
    if root.is_file():
        yield root
        return
    for path in sorted(root.glob("**/*" if recursive else "*")):
        if path.is_file() and path.suffix.lower() in HTML_SUFFIXES:
            yield path


def extract_from_file(path: Path, sink: CharCollector, skip_text: bool) -> None:
    try:
        source = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        print(f"warning: {path}: not valid UTF-8, skipped", file=sys.stderr)
        return
    except OSError as exc:
        print(f"warning: cannot read {path}: {exc}", file=sys.stderr)
        return

    parser = MathExtractor(sink, skip_text=skip_text)
    parser.feed(source)
    parser.close()


def write_output(text: str, output: Optional[Path]) -> None:
    """Write the collected characters to a file, or to standard output.

    Both paths emit exactly the same bytes -- no trailing newline, always UTF-8 -- so
    `-o FILE` and `> FILE` are interchangeable.
    """
    if text:
        text += "\n"

    if output is not None:
        try:
            output.write_text(text, encoding="utf-8")
        except OSError as exc:
            sys.exit(f"error: cannot write {output}: {exc}")
        return

    try:
        # Bytes rather than sys.stdout.write(), because the characters of a math font
        # are mostly non-ASCII and the locale encoding may not be able to represent
        # them.
        sys.stdout.flush()
        sys.stdout.buffer.write(text.encode("utf-8"))
        sys.stdout.buffer.flush()
    except BrokenPipeError:
        # The reader went away (`... | head`). Point the real file descriptor at the
        # null device, so the interpreter's own flush on exit cannot fail again and
        # print "Exception ignored" to stderr.
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        sys.exit(1)
    except OSError as exc:
        sys.exit(f"error: cannot write standard output: {exc}")


def main(argv: Optional[Iterable[str]] = None) -> int:
    ap = argparse.ArgumentParser(
        description="Extract the distinct characters used inside <math> "
        "elements of HTML files, for hb-subset --text-file."
    )
    ap.add_argument(
        "input", type=Path, help="HTML file, or directory containing HTML files"
    )
    ap.add_argument(
        "-r",
        "--recursive",
        action="store_true",
        help="descend into subdirectories of the input directory",
    )
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        metavar="FILE",
        help="text file to write, all input merged into it (default: standard output)",
    )
    ap.add_argument(
        "-s",
        "--skip-text",
        action="store_true",
        help="ignore the contents of <mtext> elements",
    )
    args = ap.parse_args(argv)

    if not args.input.exists():
        ap.error(f"{args.input}: no such file or directory")
    if args.recursive and args.input.is_file():
        print("warning: --recursive has no effect on a single file", file=sys.stderr)

    sink = CharCollector()
    n_files = 0
    for path in iter_html_files(args.input, args.recursive):
        n_files += 1
        extract_from_file(path, sink, args.skip_text)
    text = sink.text()

    leftovers = sorted(sink.leftovers)
    if leftovers:
        shown = ", ".join(leftovers[:10])
        if len(leftovers) > 10:
            shown += ", ..."
        print(
            f"warning: possibly unresolved entity references: {shown}", file=sys.stderr
        )

    variants = sorted(sink.variants)
    if variants:
        print(
            f"warning: untransformed mathvariant values: {', '.join(variants)}; "
            "characters these substitute may be missing from the subset",
            file=sys.stderr,
        )

    write_output(text, args.output)

    destination = args.output if args.output is not None else "<stdout>"
    print(
        f"{n_files} file(s) -> {destination}: "
        f"{len(text)} distinct code points from {sink.total} characters",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
