from pytest import raises
from math_core import LatexError, LatexToMathML


def test_identifier():
    converter = LatexToMathML(pretty_print=False)
    assert (
        converter.convert_with_local_counter("x", block=False)
        == "<math><mi>x</mi></math>"
    )
    assert (
        converter.convert_with_local_counter("x", block=True)
        == '<math display="block"><mi>x</mi></math>'
    )


def test_exception():
    converter = LatexToMathML(pretty_print=False)
    with raises(LatexError):
        converter.convert_with_local_counter(r"\nonexistentcommand", block=False)


def test_macros():
    converter = LatexToMathML(pretty_print=False, macros={"ab": "ab"})
    assert (
        converter.convert_with_local_counter(r"\ab", block=False)
        == "<math><mrow><mi>a</mi><mi>b</mi></mrow></math>"
    )


def test_global_counter():
    converter = LatexToMathML()
    output = converter.convert_with_global_counter(
        r"\begin{align}x\end{align}", block=True
    )
    assert "(1)" in output
    output = converter.convert_with_global_counter(
        r"\begin{align}y\end{align}", block=True
    )
    assert "(2)" in output

    converter.reset_global_counter()
    output = converter.convert_with_global_counter(
        r"\begin{align}z\end{align}", block=True
    )
    assert "(1)" in output
