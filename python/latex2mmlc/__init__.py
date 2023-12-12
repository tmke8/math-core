from enum import Enum

# maturin is configured to put the compiled library in `_latex2mmlc_rust`
from ._latex2mmlc_rust import convert_latex as _convert_latex

__all__ = ["Display", "convert_latex"]


class Display(Enum):
    INLINE = False
    BLOCK = True


def convert_latex(latex: str, display: Display = Display.INLINE, pretty: bool = True) -> str:
    return _convert_latex(latex, display.value, pretty)
