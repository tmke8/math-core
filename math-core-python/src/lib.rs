use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyString};
use pyo3::{create_exception, intern};

use math_core::{Config, Display, latex_to_mathml};

create_exception!(_math_core_rust, LatexError, PyException);

/// Convert LaTeX equation to MathML.
#[pyfunction]
#[pyo3(signature = (latex, block, config))]
fn convert_latex<'a>(
    py: Python<'a>,
    latex: &str,
    block: bool,
    config: Option<&Bound<'a, PyAny>>,
) -> PyResult<Bound<'a, PyString>> {
    let config = if let Some(cfg) = config {
        // We support duck-typing for the passed-in config object.
        Config {
            pretty: cfg
                .getattr(intern!(py, "pretty"))?
                .downcast_into::<PyBool>()?
                .is_true(),
            ..Default::default()
        }
    } else {
        Default::default()
    };
    let result = latex_to_mathml(
        latex,
        if block {
            Display::Block
        } else {
            Display::Inline
        },
        &config,
    )
    .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?;
    Ok(PyString::new(py, &result))
}

/// A Python module implemented in Rust.
#[pymodule]
fn _math_core_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LatexError", m.py().get_type::<LatexError>())?;
    m.add_function(wrap_pyfunction!(convert_latex, m)?)?;
    Ok(())
}
