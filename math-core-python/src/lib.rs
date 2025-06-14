use std::sync::RwLock;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

use math_core::{MathCoreConfig, MathDisplay};
use rustc_hash::FxHashMap;

create_exception!(_math_core_rust, LatexError, PyException);

#[pyclass(frozen)]
struct LatexToMathML {
    inner: RwLock<math_core::LatexToMathML>,
}

#[pyclass(eq, eq_int, rename_all = "UPPERCASE")]
#[derive(PartialEq)]
enum PrettyPrint {
    Never,
    Always,
    Auto,
}

#[pymethods]
impl LatexToMathML {
    #[new]
    #[pyo3(signature = (*, pretty_print=&PrettyPrint::Never, macros=None))]
    fn new(pretty_print: &PrettyPrint, macros: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let pretty_print = match pretty_print {
            PrettyPrint::Never => math_core::PrettyPrint::Never,
            PrettyPrint::Always => math_core::PrettyPrint::Always,
            PrettyPrint::Auto => math_core::PrettyPrint::Auto,
        };
        let config = MathCoreConfig {
            pretty_print,
            macros: if let Some(macros_dict) = macros {
                dict_to_hashmap(macros_dict)?
            } else {
                Default::default()
            },
            ..Default::default()
        };

        Ok(LatexToMathML {
            inner: RwLock::new(
                math_core::LatexToMathML::new(&config)
                    .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?,
            ),
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
        let result = self
            .inner
            .write()
            .map_err(|_| LatexError::new_err("Failed to acquire write lock"))?
            .convert_with_global_counter(
                latex,
                if displaystyle {
                    MathDisplay::Block
                } else {
                    MathDisplay::Inline
                },
            )
            .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?;
        Ok(PyString::new(py, &result))
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_local_counter<'a>(
        &self,
        latex: &str,
        displaystyle: bool,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyString>> {
        let result = self
            .inner
            .read()
            .map_err(|_| LatexError::new_err("Failed to acquire read lock"))?
            .convert_with_local_counter(
                latex,
                if displaystyle {
                    MathDisplay::Block
                } else {
                    MathDisplay::Inline
                },
            )
            .map_err(|latex_error| LatexError::new_err(latex_error.to_string()))?;
        Ok(PyString::new(py, &result))
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
    m.add_class::<PrettyPrint>()?;
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
