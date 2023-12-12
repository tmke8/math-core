use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use latex2mmlc::{latex_to_mathml, Display};

create_exception!(_latex2mmlc_rust, LatexError, PyException);

/// Convert LaTeX equation to MathML.
#[pyfunction]
fn convert_latex(latex: &str, block: bool, pretty: bool) -> PyResult<String> {
    latex_to_mathml(
        latex,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        pretty,
    )
    .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))
}

/// A Python module implemented in Rust.
#[pymodule]
fn _latex2mmlc_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("LatexError", _py.get_type::<LatexError>())?;
    m.add_function(wrap_pyfunction!(convert_latex, m)?)?;
    Ok(())
}
