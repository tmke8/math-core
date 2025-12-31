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
    converter = LatexToMathML.with_config(pretty_print="never")
    assert isinstance(converter, LatexToMathML)
    err = converter.convert_with_local_counter(
        r"\nonexistentcommand", displaystyle=False
    )
    assert isinstance(err, LatexError)
    assert err.location == 0
    match converter.convert_with_local_counter(r"öäüßx^", displaystyle=False):
        case LatexError(message, location):
            assert location == 6
            assert "argument" in message
        case _:
            assert False, "Expected LatexError"

    with raises(ValueError):
        _ = LatexToMathML.with_config(pretty_print="sometimes")  # type: ignore


def test_macros():
    converter = LatexToMathML.with_config(pretty_print="never", macros={"ab": "cd"})
    assert isinstance(converter, LatexToMathML)
    assert (
        converter.convert_with_local_counter(r"\ab", displaystyle=False)
        == "<math><mi>c</mi><mi>d</mi></math>"
    )


def test_global_counter():
    converter = LatexToMathML()
    output = converter.convert_with_global_counter(
        r"\begin{align}x\end{align}", displaystyle=True
    )
    assert isinstance(output, str)
    assert "(1)" in output
    output = converter.convert_with_global_counter(
        r"\begin{align}y\end{align}", displaystyle=True
    )
    assert isinstance(output, str)
    assert "(2)" in output

    converter.reset_global_counter()
    output = converter.convert_with_global_counter(
        r"\begin{align}z\end{align}", displaystyle=True
    )
    assert isinstance(output, str)
    assert "(1)" in output


def test_signature():
    assert (
        str(inspect.signature(LatexToMathML.__init__)) == "(self, /, *args, **kwargs)"
    )
    assert (
        str(inspect.signature(LatexToMathML.with_config))
        == "(*, pretty_print='never', macros=None, xml_namespace=False, raise_on_error=True)"
    )
    converter = LatexToMathML()
    assert (
        str(inspect.signature(converter.convert_with_local_counter))
        == "(latex, *, displaystyle)"
    )


def test_xml():
    converter = LatexToMathML.with_config(xml_namespace=True)
    assert isinstance(converter, LatexToMathML)
    assert (
        converter.convert_with_local_counter("x", displaystyle=False)
        == '<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>'
    )


def test_continue_on_error():
    converter = LatexToMathML.with_config(raise_on_error=False)
    assert isinstance(converter, LatexToMathML)
    assert (
        converter.convert_with_local_counter("\\asdf <b>", displaystyle=False)
        == r'<span class="math-core-error" title="0: Unknown command &quot;\asdf&quot;."><code>\asdf &lt;b&gt;</code></span>'
    )
    assert (
        converter.convert_with_local_counter("\\begin{\"} '&", displaystyle=True)
        == '<p class="math-core-error" title="7: Disallowed character in text group: \'&quot;\'."><code>\\begin{"} \'&amp;</code></p>'
    )
