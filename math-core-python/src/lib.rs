use std::sync::RwLock;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

use math_core::{MathCoreConfig, MathDisplay, PrettyPrint};
use rustc_hash::FxHashMap;

create_exception!(_math_core_rust, LatexError, PyException);

#[pyclass(frozen)]
struct LatexToMathML {
    inner: RwLock<math_core::LatexToMathML>,
    raise_on_error: bool,
}

#[pymethods]
impl LatexToMathML {
    #[new]
    #[pyo3(signature = (*, pretty_print="never", macros=None, xml_namespace=false, raise_on_error=true))]
    fn new(
        pretty_print: &str,
        macros: Option<&Bound<'_, PyDict>>,
        xml_namespace: bool,
        raise_on_error: bool,
    ) -> PyResult<Self> {
        let pretty_print = match pretty_print {
            "never" => PrettyPrint::Never,
            "always" => PrettyPrint::Always,
            "auto" => PrettyPrint::Auto,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid pretty_print value: '{}'. Must be 'never', 'always', or 'auto'.",
                    pretty_print
                )));
            }
        };
        let config = MathCoreConfig {
            pretty_print,
            macros: if let Some(macros_dict) = macros {
                dict_to_hashmap(macros_dict)?
            } else {
                Default::default()
            },
            xml_namespace,
        };

        Ok(LatexToMathML {
            inner: RwLock::new(
                math_core::LatexToMathML::new(&config)
                    .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?,
            ),
            raise_on_error,
        })
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_global_counter<'a>(
        &self,
        latex: &str,
        displaystyle: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyString>> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self
            .inner
            .write()
            .map_err(|_| LatexError::new_err("Failed to acquire write lock"))?
            .convert_with_global_counter(latex, display)
        {
            Err(latex_error) => {
                if self.raise_on_error {
                    Err(LatexError::new_err(latex_error.to_string()))
                } else {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                }
            }
            Ok(output) => Ok(PyString::new(py, &output)),
        }
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_local_counter<'a>(
        &self,
        latex: &str,
        displaystyle: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyString>> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self
            .inner
            .write()
            .map_err(|_| LatexError::new_err("Failed to acquire read lock"))?
            .convert_with_local_counter(latex, display)
        {
            Err(latex_error) => {
                if self.raise_on_error {
                    Err(LatexError::new_err(latex_error.to_string()))
                } else {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                }
            }
            Ok(output) => Ok(PyString::new(py, &output)),
        }
    }

    fn reset_global_counter(&self) -> PyResult<()> {
        self.inner
            .write()
            .map_err(|_| LatexError::new_err("Failed to acquire write lock"))?
            .reset_global_counter();
        Ok(())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _math_core_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LatexError", m.py().get_type::<LatexError>())?;
    m.add_class::<LatexToMathML>()?;
    Ok(())
}

fn dict_to_hashmap(dict: &Bound<'_, PyDict>) -> PyResult<FxHashMap<String, String>> {
    let mut map = FxHashMap::with_capacity_and_hasher(dict.len(), Default::default());

    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;
        let value_str = value.extract::<String>()?;
        map.insert(key_str, value_str);
    }

    Ok(map)
}
