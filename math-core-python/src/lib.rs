use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyString};
use pyo3::{create_exception, intern};

use math_core::{Config, Display};

create_exception!(_math_core_rust, LatexError, PyException);

#[pyclass]
struct LatexToMathML {
    inner: Box<math_core::LatexToMathML>,
}

#[pymethods]
impl LatexToMathML {
    #[new]
    fn new<'a>(config: Option<&Bound<'a, PyAny>>, py: Python<'a>) -> PyResult<Self> {
        let config = if let Some(cfg) = config {
            // We support duck-typing for the passed-in config object.
            Config {
                pretty_print: cfg
                    .getattr(intern!(py, "pretty_print"))?
                    .downcast_into::<PyBool>()?
                    .is_true(),
                ..Default::default()
            }
        } else {
            Default::default()
        };
        Ok(LatexToMathML {
            inner: Box::new(math_core::LatexToMathML::new(&config).unwrap()),
        })
    }

    /// Convert LaTeX equation to MathML.
    fn convert<'a>(
        &mut self,
        latex: &str,
        block: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyString>> {
        let result = self
            .inner
            .convert_with_global_counter(
                latex,
                if block {
                    Display::Block
                } else {
                    Display::Inline
                },
            )
            .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?;
        Ok(PyString::new(py, &result))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _math_core_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LatexError", m.py().get_type::<LatexError>())?;
    m.add_class::<LatexToMathML>()?;
    Ok(())
}
