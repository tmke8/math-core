class LatexToMathML:
    r"""Convert LaTeX to MathML Core.

    Args:
        pretty_print: If True, the output will be formatted with indentation and newlines.
        macros: A dictionary of LaTeX macros to be used in the conversion. For example,
            `{"d": r"\mathrm{d}"}` will replace `\d` with `\mathrm{d}` in the LaTeX input.
    """
    def __init__(
        self, pretty_print: bool = False, macros: dict[str, str] | None = None
    ) -> None: ...
    def convert_with_global_counter(self, latex: str, block: bool) -> str:
        """Convert LaTeX to MathML with a global counter for environments like `align`."""
    def convert_with_local_counter(self, latex: str, block: bool) -> str:
        """Convert LaTeX to MathML with a local counter for environments like `align`."""
    def reset_global_counter(self) -> None:
        """Reset the global counter for environments like `align`."""

class LatexError(Exception):
    """Exception raised for errors in the LaTeX to MathML conversion process."""
