use std::sync::RwLock;

use pyo3::exceptions::PyException;
use pyo3::types::{PyDict, PyString};
use pyo3::{create_exception, prelude::*};

use math_core::{MathCoreConfig, MathDisplay, PrettyPrint};

create_exception!(_math_core_rust, LatexError, PyException);
create_exception!(_math_core_rust, LockError, PyException);

#[pyclass(frozen)]
struct LatexToMathML {
    inner: RwLock<math_core::LatexToMathML>,
    continue_on_error: bool,
}

#[pymethods]
impl LatexToMathML {
    #[new]
    #[pyo3(signature = (*, pretty_print="never", macros=None, xml_namespace=false, continue_on_error=false, ignore_unknown_commands=false, annotation=false))]
    fn new(
        pretty_print: &str,
        macros: Option<&Bound<'_, PyDict>>,
        xml_namespace: bool,
        continue_on_error: bool,
        ignore_unknown_commands: bool,
        annotation: bool,
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
                dict_to_tuple_vec(macros_dict)?
            } else {
                Default::default()
            },
            xml_namespace,
            ignore_unknown_commands,
            annotation,
        };

        let inner = math_core::LatexToMathML::new(config);
        match inner {
            Ok(inner) => Ok(LatexToMathML {
                inner: RwLock::new(inner),
                continue_on_error,
            }),
            Err((latex_error, idx, source)) => {
                let source_name = format!("macro{}", idx);
                let err = latex_error.to_message(&source_name, &source);
                Err(LatexError::new_err(err))
            }
        }
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
            .map_err(|_| LockError::new_err("Failed to acquire write lock"))?
            .convert_with_global_counter(latex, display)
        {
            Err(mut latex_error) => {
                // Rust uses byte offsets, but Python uses character offsets.
                latex_error.0.start = byte_offset_to_char_offset(latex, latex_error.0.start);
                if self.continue_on_error {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                } else {
                    Err(LatexError::new_err(latex_error.to_string()))
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
            .map_err(|_| LockError::new_err("Failed to acquire read lock"))?
            .convert_with_local_counter(latex, display)
        {
            Err(mut latex_error) => {
                // Rust uses byte offsets, but Python uses character offsets.
                latex_error.0.start = byte_offset_to_char_offset(latex, latex_error.0.start);
                if self.continue_on_error {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                } else {
                    Err(LatexError::new_err(latex_error.to_string()))
                }
            }
            Ok(output) => Ok(PyString::new(py, &output)),
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
    m.add("LatexError", m.py().get_type::<LatexError>())?;
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
