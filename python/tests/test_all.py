from pytest import raises
from math_core import Config, LatexError, LatexToMathML


def test_identifier():
    config = Config(pretty_print=False)
    converter = LatexToMathML(config)
    assert converter.convert("x", block=False) == "<math><mi>x</mi></math>"
    assert (
        converter.convert("x", block=True) == '<math display="block"><mi>x</mi></math>'
    )


def test_exception():
    config = Config(pretty_print=False)
    converter = LatexToMathML(config)
    with raises(LatexError):
        converter.convert(r"\nonexistentcommand", block=False)
