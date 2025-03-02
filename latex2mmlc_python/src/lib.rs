use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyString;

use math_core::{Display, latex_to_mathml};

create_exception!(_latex2mmlc_rust, LatexError, PyException);

/// Convert LaTeX equation to MathML.
#[pyfunction]
fn convert_latex<'a>(
    py: Python<'a>,
    latex: &str,
    block: bool,
    pretty: bool,
) -> PyResult<Bound<'a, PyString>> {
    let result = latex_to_mathml(
        latex,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        pretty,
    )
    .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?;
    Ok(PyString::new(py, &result))
}

/// A Python module implemented in Rust.
#[pymodule]
fn _latex2mmlc_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LatexError", m.py().get_type::<LatexError>())?;
    m.add_function(wrap_pyfunction!(convert_latex, m)?)?;
    Ok(())
}
