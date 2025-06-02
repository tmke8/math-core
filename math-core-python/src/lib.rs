use std::sync::Mutex;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyString};
use pyo3::{create_exception, intern};

use math_core::{Config, Display};

create_exception!(_math_core_rust, LatexError, PyException);

#[pyclass]
struct Converter {
    inner: Mutex<math_core::Converter>,
}

#[pymethods]
impl Converter {
    #[new]
    fn new<'a>(config: Option<&Bound<'a, PyAny>>, py: Python<'a>) -> PyResult<Self> {
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
        Ok(Converter {
            inner: Mutex::new(Box::new(math_core::Converter::new(&config).unwrap())),
        })
    }

    /// Convert LaTeX equation to MathML.
    fn latex_to_mathml<'a>(
        &mut self,
        latex: &str,
        block: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyString>> {
        let result = self
            .inner
            .lock()
            .map_err(|_| LatexError::new_err("Failed to acquire lock on converter"))?
            .latex_to_mathml(
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
    m.add_class::<Converter>()?;
    Ok(())
}
