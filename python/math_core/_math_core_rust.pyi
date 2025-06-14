from enum import Enum

class PrettyPrint(Enum):
    """Enum for pretty print options."""

    NEVER = ...
    """Never pretty print the MathML output."""
    ALWAYS = ...
    """Always pretty print the MathML output."""
    AUTO = ...
    """Pretty print block equations, but not inline equations."""

class LatexToMathML:
    r"""Convert LaTeX to MathML Core.

    Args:
        pretty_print: An enum value indicating whether to pretty print the MathML output.
            Options are `PrettyPrint.NEVER`, `PrettyPrint.ALWAYS`, or `PrettyPrint.AUTO`.
            `PrettyPrint.AUTO` means that all block equations will be pretty printed.
        macros: A dictionary of LaTeX macros to be used in the conversion. For example,
            `{"d": r"\mathrm{d}"}` will replace `\d` with `\mathrm{d}` in the LaTeX input.
    """
    def __init__(
        self,
        *,
        pretty_print: PrettyPrint = PrettyPrint.NEVER,
        macros: dict[str, str] | None = None,
    ) -> None: ...
    def convert_with_global_counter(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a global counter for environments like `align`."""
    def convert_with_local_counter(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a local counter for environments like `align`."""
    def reset_global_counter(self) -> None:
        """Reset the global counter for environments like `align`."""

class LatexError(Exception):
    """Exception raised for errors in the LaTeX to MathML conversion process."""
