import pytest
import latex2mmlc


def test_sum_as_string():
    assert latex2mmlc.convert_latex(r"\int") == "2"
