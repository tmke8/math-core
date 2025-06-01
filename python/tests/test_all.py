from pytest import raises
from math_core import Converter, Config, LatexError


def test_identifier():
    config = Config(pretty=False)
    converter = Converter(config)
    assert converter.latex_to_mathml("x", block=False) == "<math><mi>x</mi></math>"
    assert (
        converter.latex_to_mathml("x", block=True)
        == '<math display="block"><mi>x</mi></math>'
    )


def test_exception():
    config = Config(pretty=False)
    converter = Converter(config)
    with raises(LatexError):
        converter.latex_to_mathml(r"\nonexistentcommand", block=False)
