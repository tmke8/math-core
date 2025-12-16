import inspect

from math_core import LatexError, LatexToMathML
from pytest import raises


def test_identifier():
    converter = LatexToMathML()
    assert (
        converter.convert_with_local_counter("x", displaystyle=False)
        == "<math><mi>x</mi></math>"
    )
    assert (
        converter.convert_with_local_counter("x", displaystyle=True)
        == '<math display="block"><mi>x</mi></math>'
    )


def test_exception():
    converter = LatexToMathML(pretty_print="never")
    with raises(LatexError):
        converter.convert_with_local_counter(r"\nonexistentcommand", displaystyle=False)

    with raises(ValueError):
        _ = LatexToMathML(pretty_print="sometimes")  # type: ignore


def test_macros():
    converter = LatexToMathML(pretty_print="never", macros={"ab": "cd"})
    assert (
        converter.convert_with_local_counter(r"\ab", displaystyle=False)
        == "<math><mi>c</mi><mi>d</mi></math>"
    )


def test_global_counter():
    converter = LatexToMathML()
    output = converter.convert_with_global_counter(
        r"\begin{align}x\end{align}", displaystyle=True
    )
    assert "(1)" in output
    output = converter.convert_with_global_counter(
        r"\begin{align}y\end{align}", displaystyle=True
    )
    assert "(2)" in output

    converter.reset_global_counter()
    output = converter.convert_with_global_counter(
        r"\begin{align}z\end{align}", displaystyle=True
    )
    assert "(1)" in output


def test_signature():
    assert (
        str(inspect.signature(LatexToMathML.__init__)) == "(self, /, *args, **kwargs)"
    )
    converter = LatexToMathML()
    assert (
        str(inspect.signature(converter.convert_with_local_counter))
        == "(latex, *, displaystyle)"
    )


def test_xml():
    converter = LatexToMathML(xml_namespace=True)
    assert (
        converter.convert_with_local_counter("x", displaystyle=False)
        == '<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>'
    )


def test_continue_on_error():
    converter = LatexToMathML(raise_on_error=False)
    assert (
        converter.convert_with_local_counter("\\asdf <b>", displaystyle=False)
        == r'<span class="math-core-error" title="0: Unknown command &quot;\asdf&quot;."><code>\asdf &lt;b&gt;</code></span>'
    )
    assert (
        converter.convert_with_local_counter("\\begin{\"} '&", displaystyle=True)
        == '<p class="math-core-error" title="7: Disallowed character in text group: \'&quot;\'."><code>\\begin{"} \'&amp;</code></p>'
    )
