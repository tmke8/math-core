import math_core


def test_integral():
    assert math_core.convert_latex(r"\int") == "<math><mo>∫</mo></math>"
