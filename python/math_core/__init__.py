from dataclasses import dataclass

# maturin is configured to put the compiled library in `_math_core_rust`
from ._math_core_rust import Converter, LatexError

__all__ = ["Config", "Converter", "LatexError"]


@dataclass
class Config:
    pretty: bool = True
