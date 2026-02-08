use std::sync::RwLock;

use pyo3::exceptions::PyException;
use pyo3::types::{PyDict, PyString, PyType};
use pyo3::{IntoPyObjectExt, create_exception, prelude::*};

use math_core::{MathCoreConfig, MathDisplay, PrettyPrint};

create_exception!(_math_core_rust, LockError, PyException);

#[pyclass(frozen)]
struct LatexError {
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    location: usize,
    #[pyo3(get)]
    context: Option<String>,
}

#[pymethods]
impl LatexError {
    #[classattr]
    fn __match_args__() -> (String, String, String) {
        (
            "message".to_string(),
            "location".to_string(),
            "context".to_string(),
        )
    }
}

#[pyclass(frozen)]
struct LatexToMathML {
    inner: RwLock<math_core::LatexToMathML>,
    continue_on_error: bool,
}

#[pymethods]
impl LatexToMathML {
    #[classmethod]
    #[pyo3(signature = (*, pretty_print="never", macros=None, xml_namespace=false, continue_on_error=false, ignore_unknown_commands=false))]
    fn with_config<'a>(
        _cls: &Bound<'_, PyType>,
        pretty_print: &str,
        macros: Option<&Bound<'_, PyDict>>,
        xml_namespace: bool,
        continue_on_error: bool,
        ignore_unknown_commands: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
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
                dict_to_tuple_vec(macros_dict)?
            } else {
                Default::default()
            },
            xml_namespace,
            ignore_unknown_commands,
        };

        let inner = math_core::LatexToMathML::new(config);
        match inner {
            Ok(inner) => LatexToMathML {
                inner: RwLock::new(inner),
                continue_on_error,
            }
            .into_bound_py_any(py),
            Err(latex_error) => LatexError {
                message: latex_error.0.to_string(),
                location: latex_error.0.0.start,
                context: Some(latex_error.1),
            }
            .into_bound_py_any(py),
        }
    }

    #[new]
    fn new() -> Self {
        LatexToMathML {
            inner: RwLock::new(math_core::LatexToMathML::default()),
            continue_on_error: Default::default(),
        }
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_global_counter<'a>(
        &self,
        latex: &str,
        displaystyle: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self
            .inner
            .write()
            .map_err(|_| LockError::new_err("Failed to acquire write lock"))?
            .convert_with_global_counter(latex, display)
        {
            Err(mut latex_error) => {
                // Rust uses byte offsets, but Python uses character offsets.
                latex_error.0.start = byte_offset_to_char_offset(latex, latex_error.0.start);
                if self.continue_on_error {
                    Ok(PyString::new(py, &latex_error.to_html(latex, display, None)).into_any())
                } else {
                    LatexError {
                        message: latex_error.to_string(),
                        location: latex_error.0.start,
                        context: None,
                    }
                    .into_bound_py_any(py)
                }
            }
            Ok(output) => Ok(PyString::new(py, &output).into_any()),
        }
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_local_counter<'a>(
        &self,
        latex: &str,
        displaystyle: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self
            .inner
            .write()
            .map_err(|_| LockError::new_err("Failed to acquire read lock"))?
            .convert_with_local_counter(latex, display)
        {
            Err(mut latex_error) => {
                // Rust uses byte offsets, but Python uses character offsets.
                latex_error.0.start = byte_offset_to_char_offset(latex, latex_error.0.start);
                if self.continue_on_error {
                    Ok(PyString::new(py, &latex_error.to_html(latex, display, None)).into_any())
                } else {
                    LatexError {
                        message: latex_error.to_string(),
                        location: latex_error.0.start,
                        context: None,
                    }
                    .into_bound_py_any(py)
                }
            }
            Ok(output) => Ok(PyString::new(py, &output).into_any()),
        }
    }

    fn reset_global_counter(&self) -> PyResult<()> {
        self.inner
            .write()
            .map_err(|_| LockError::new_err("Failed to acquire write lock"))?
            .reset_global_counter();
        Ok(())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _math_core_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LockError", m.py().get_type::<LockError>())?;
    m.add_class::<LatexError>()?;
    m.add_class::<LatexToMathML>()?;
    Ok(())
}

fn dict_to_tuple_vec(dict: &Bound<'_, PyDict>) -> PyResult<Vec<(String, String)>> {
    let mut vec = Vec::with_capacity(dict.len());

    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;
        let value_str = value.extract::<String>()?;
        vec.push((key_str, value_str));
    }

    Ok(vec)
}

/// Convert a byte offset in a UTF-8 string to a character offset.
///
/// Panics if the byte offset is not on a character boundary.
fn byte_offset_to_char_offset(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset].chars().count()
}
