from dataclasses import dataclass

# maturin is configured to put the compiled library in `_math_core_rust`
from ._math_core_rust import LatexError, LatexToMathML

__all__ = ["Config", "LatexError", "LatexToMathML"]


@dataclass
class Config:
    pretty: bool = True
