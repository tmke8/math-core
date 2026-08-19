from typing import Literal

class LatexToMathML:
    """Convert LaTeX to MathML Core."""
    def __init__(
        self,
        *,
        pretty_print: Literal["never", "always", "auto"] = "never",
        macros: dict[str, str] | None = None,
        xml_namespace: bool = False,
        continue_on_error: bool = False,
        ignore_unknown_commands: bool = False,
        annotation: bool = False,
        allow_unreliable_rendering: bool = False,
        global_group: bool = False,
        fancy_error: bool = True,
        unicode_substitution: Literal["never", "conventional"] = "conventional",
        max_expansions: int = 1000,
    ) -> None:
        r"""Create a LatexToMathML converter with the specified configuration.

        Args:
            pretty_print: A string indicating whether to pretty print the MathML output.
                Allowed values are:

                * "never": Never pretty print the MathML output.
                * "always": Always pretty print the MathML output.
                * "auto": Pretty print block equations, but not inline equations.

            macros: A dictionary of LaTeX macros to be used in the conversion. For
                example, ``{"d": r"\mathrm{d}"}`` will replace ``\d`` with
                ``\mathrm{d}`` in the LaTeX input.

            xml_namespace: A boolean indicating whether to include ``xmlns="..."``.

            continue_on_error: A boolean indicating whether to return an error for
                conversion errors. If conversion fails and this is ``True``, an HTML
                snippet describing the error will be returned, instead of returning
                ``LatexError``.

            ignore_unknown_commands: A boolean indicating whether to ignore unknown
                LaTeX commands. If ``True``, unknown commands will be displayed as red
                text and the conversion will continue, instead of returning an error.

            global_group: A boolean indicating whether to run the conversion in the
                global group. If ``True``, commands defined at the top level with
                ``\newcommand`` (and related commands) stay defined for subsequent
                calls to ``convert_with_global_state``. If ``False`` (the default),
                such definitions are local to the snippet which contains them, which
                is what LaTeX does for constructs like ``\begin{equation}``.

            fancy_error: A boolean indicating whether to render errors as rich Ariadne
                reports. If ``True`` (the default), the ``LatexError`` message will
                contain a formatted diagnostic with source spans. If ``False``, a
                compact plain-text message is used instead.

            unicode_substitution: A string indicating whether to substitute certain
                character combinations with a single Unicode symbol.

            max_expansions: The number of custom command expansions allowed in one
                snippet. This limit exists because a macro may expand to itself,
                directly or indirectly, in which case the expansion would never end.
        """
    def convert_with_global_state(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a global counter for equation numbering."""
    def convert_with_local_state(self, latex: str, *, displaystyle: bool) -> str:
        """Convert LaTeX to MathML with a local counter for equation numbering."""
    def convert_all(self, snippets: list[tuple[str, bool]]) -> list[str]:
        """Convert a collection of LaTeX snippets to MathML.

        In contrast to the other conversion methods, this one resolves *forward
        references* correctly, meaning that a snippet can refer to an equation which
        is only defined in a later snippet. This is why all snippets of a document
        have to be passed in at once.

        The conversion does not touch the global state; it uses a fresh state for the
        whole batch.

        Args:
            snippets: A list of ``(latex, displaystyle)`` pairs, where ``latex`` is the
                LaTeX code of the snippet and ``displaystyle`` indicates whether it is
                a block equation.

        Returns:
            The MathML for each snippet, in the order the snippets were given in.

        Raises:
            LatexError: If any snippet fails to convert, unless the converter was
                created with ``continue_on_error=True``, in which case an HTML snippet
                describing the error takes the place of that snippet's MathML.
        """
    def reset_global_state(self) -> None:
        """Reset the global equation counter for environments like ``align``."""

class LatexError(Exception):
    """Raised when a LaTeX conversion error occurs."""

class LockError(Exception):
    """Raised when a lock cannot be acquired."""
