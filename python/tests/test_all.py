import math_core


def test_integral():
    assert (
        math_core.convert_latex(r"\int")
        == '<math xmlns="http://www.w3.org/1998/Math/MathML" display="inline">\n<mo>∫</mo>\n</math>'
    )
