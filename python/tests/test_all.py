import latex2mmlc


def test_integral():
    assert (
        latex2mmlc.convert_latex(r"\int")
        == '<math xmlns="http://www.w3.org/1998/Math/MathML" display="inline">\n<mo>∫</mo>\n</math>'
    )
