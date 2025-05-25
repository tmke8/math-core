from dataclasses import dataclass
from enum import Enum
from typing import Optional

# maturin is configured to put the compiled library in `_latex2mmlc_rust`
from ._latex2mmlc_rust import convert_latex as _convert_latex

__all__ = ["Config", "Display", "convert_latex"]


class Display(Enum):
    INLINE = False
    BLOCK = True


@dataclass
class Config:
    pretty: bool = True


def convert_latex(
    latex: str, display: Display = Display.INLINE, config: Optional[Config] = None
) -> str:
    return _convert_latex(latex, display.value, config)
