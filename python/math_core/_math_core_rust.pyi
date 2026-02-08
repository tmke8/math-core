from typing import Literal

from typing_extensions import Self

class LatexToMathML:
    """Convert LaTeX to MathML Core."""
    def __init__(self) -> None: ...
    @classmethod
    def with_config(
        cls,
        *,
        pretty_print: Literal["never", "always", "auto"] = "never",
        macros: dict[str, str] | None = None,
        xml_namespace: bool = False,
        continue_on_error: bool = False,
        ignore_unknown_commands: bool = False,
    ) -> Self | LatexError:
        r"""Create a LatexToMathML converter with the specified configuration.

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

            continue_on_error: A boolean indicating whether to return an error for
                conversion errors. If conversion fails and this is ``True``, an HTML
                snippet describing the error will be returned, instead of returning
                ``LatexError``.

            ignore_unknown_commands: A boolean indicating whether to ignore unknown
                LaTeX commands. If ``True``, unknown commands will be displayed as red text
                and the conversion will continue, instead of returning an error.
        """
    def convert_with_global_counter(
        self, latex: str, *, displaystyle: bool
    ) -> str | LatexError:
        """Convert LaTeX to MathML with a global counter for equation numbering."""
    def convert_with_local_counter(
        self, latex: str, *, displaystyle: bool
    ) -> str | LatexError:
        """Convert LaTeX to MathML with a local counter for equation numbering."""
    def reset_global_counter(self) -> None:
        """Reset the global equation counter for environments like ``align``."""

class LatexError:
    __match_args__ = ("message", "location", "context")
    message: str
    location: int
    context: str | None

class LockError(Exception):
    """Raised when a lock cannot be acquired."""
