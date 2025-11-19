from typing import Literal

class LatexToMathML:
    r"""Convert LaTeX to MathML Core.

    Args:
        pretty_print: A string indicating whether to pretty print the MathML output.
            Allowed values are:

            * "never": Never pretty print the MathML output.
            * "always": Always pretty print the MathML output.
            * "auto": Pretty print block equations, but not inline equations.

        macros: A dictionary of LaTeX macros to be used in the conversion. For example,
            ``{"d": r"\mathrm{d}"}`` will replace ``\d`` with ``\mathrm{d}`` in the
            LaTeX input.

        xml_namespace: A boolean indicating whether to include ``xmlns="..."``.

        raise_on_error: A boolean indicating whether to raise an exception for
            conversion errors. If conversion fails and this is ``False``, an HTML
            snippet describing the error will be returned.
    """
    def __init__(
        self,
        *,
        pretty_print: Literal["never", "always", "auto"] = "never",
        macros: dict[str, str] | None = None,
        xml_namespace: bool = False,
        raise_on_error: bool = True,
    ) -> None: ...
    def convert_with_global_counter(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a global counter for equation numbering."""
    def convert_with_local_counter(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a local counter for equation numbering."""
    def reset_global_counter(self) -> None:
        """Reset the global equation counter for environments like ``align``."""

class LatexError(Exception):
    """Exception raised for errors in the LaTeX to MathML conversion process."""
