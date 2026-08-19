use std::borrow::Cow;
use std::sync::RwLock;

use pyo3::exceptions::PyException;
use pyo3::types::{PyDict, PyList, PyString};
use pyo3::{create_exception, prelude::*};

use math_core::{CssClassNames, MathCoreConfig, MathDisplay, MaxExpansions, PrettyPrint};

create_exception!(_math_core_rust, LatexError, PyException);
create_exception!(_math_core_rust, LockError, PyException);

#[pyclass(frozen)]
struct LatexToMathML {
    inner: RwLock<math_core::LatexToMathML>,
    continue_on_error: bool,
    fancy_error: bool,
}

fn render_fancy_error(error: &math_core::LatexError, source_name: &str, input: &str) -> String {
    let report = error.to_report(source_name, true);
    let mut buf = vec![b'\n'];
    report
        .write((source_name, ariadne::Source::from(input)), &mut buf)
        .expect("failed to write report");
    String::from_utf8(buf).expect("report should be valid UTF-8")
}

#[pymethods]
impl LatexToMathML {
    #[new]
    #[pyo3(signature = (*,
        pretty_print="never",
        macros=None,
        xml_namespace=false,
        continue_on_error=false,
        ignore_unknown_commands=false,
        annotation=false,
        allow_unreliable_rendering=false,
        global_group=false,
        fancy_error=true,
        unicode_substitution="conventional",
        max_expansions=MaxExpansions::default().0))]
    fn new(
        pretty_print: &str,
        macros: Option<&Bound<'_, PyDict>>,
        xml_namespace: bool,
        continue_on_error: bool,
        ignore_unknown_commands: bool,
        annotation: bool,
        allow_unreliable_rendering: bool,
        global_group: bool,
        fancy_error: bool,
        unicode_substitution: &str,
        max_expansions: u32,
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
        let unicode_substitution = match unicode_substitution {
            "conventional" => math_core::UnicodeSubstitution::Conventional,
            "never" => math_core::UnicodeSubstitution::Never,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid unicode_substitution value: '{}'. Must be 'conventional' or 'never'.",
                    unicode_substitution
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
            allow_unreliable_rendering,
            global_group,
            unicode_substitution,
            css_classes: CssClassNames::default(),
            indentation: math_core::Indentation::default(),
            max_expansions: MaxExpansions(max_expansions),
        };

        let inner = math_core::LatexToMathML::new(config);
        match inner {
            Ok(inner) => Ok(LatexToMathML {
                inner: RwLock::new(inner),
                continue_on_error,
                fancy_error,
            }),
            Err((latex_error, idx, source)) => {
                if fancy_error {
                    let source_name = format!("macro{idx}");
                    Err(LatexError::new_err(render_fancy_error(
                        &latex_error,
                        &source_name,
                        &source,
                    )))
                } else {
                    let mut err = format!("macro{idx}:");
                    latex_error.to_message(&mut err, &source);
                    Err(LatexError::new_err(err))
                }
            }
        }
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_global_state<'a>(
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
            .convert_with_global_state(latex, display)
        {
            Err(latex_error) => {
                if self.continue_on_error {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                } else {
                    Err(conversion_error(
                        &latex_error,
                        latex,
                        self.fancy_error,
                        None,
                    ))
                }
            }
            Ok(output) => Ok(PyString::new(py, &output.mathml)),
        }
    }

    /// Convert LaTeX equation to MathML.
    #[pyo3(signature = (latex, *, displaystyle))]
    fn convert_with_local_state<'a>(
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
            .convert_with_local_state(latex, display)
        {
            Err(latex_error) => {
                if self.continue_on_error {
                    Ok(PyString::new(
                        py,
                        &latex_error.to_html(latex, display, None),
                    ))
                } else {
                    Err(conversion_error(
                        &latex_error,
                        latex,
                        self.fancy_error,
                        None,
                    ))
                }
            }
            Ok(output) => Ok(PyString::new(py, &output.mathml)),
        }
    }

    /// Convert a collection of LaTeX snippets to MathML.
    ///
    /// In contrast to the other conversion methods, this one resolves *forward references*
    /// correctly, meaning that a snippet can refer to an equation which is only defined in a
    /// later snippet. This is why all snippets of a document have to be passed in at once.
    ///
    /// The conversion does not touch the global state; it uses a fresh state for the whole batch.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 can only extract into an owned collection"
    )]
    fn convert_all<'a>(
        &self,
        snippets: Vec<(String, bool)>,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyList>> {
        let inputs: Vec<(&str, MathDisplay)> = snippets
            .iter()
            .map(|(latex, displaystyle)| {
                (
                    latex.as_str(),
                    if *displaystyle {
                        MathDisplay::Block
                    } else {
                        MathDisplay::Inline
                    },
                )
            })
            .collect();

        let outputs = self
            .inner
            .read()
            .map_err(|_| LockError::new_err("Failed to acquire read lock"))?
            .convert_all(&inputs)
            .into_iter()
            .enumerate()
            .map(|(index, result)| {
                let (latex, display) = inputs[index];
                match result {
                    Err(latex_error) => {
                        if self.continue_on_error {
                            Ok(latex_error.to_html(latex, display, None))
                        } else {
                            Err(conversion_error(
                                &latex_error,
                                latex,
                                self.fancy_error,
                                Some(index),
                            ))
                        }
                    }
                    Ok(output) => Ok(output.mathml),
                }
            })
            .collect::<PyResult<Vec<String>>>()?;
        PyList::new(py, outputs)
    }

    fn reset_global_state(&self) -> PyResult<()> {
        self.inner
            .write()
            .map_err(|_| LockError::new_err("Failed to acquire write lock"))?
            .reset_global_state();
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

/// Build the `LatexError` exception which is raised for a failed conversion.
///
/// `index` is the position of the failing snippet within a batch, if there is a batch.
fn conversion_error(
    error: &math_core::LatexError,
    latex: &str,
    fancy_error: bool,
    index: Option<usize>,
) -> PyErr {
    let source_name = match index {
        Some(index) => Cow::Owned(format!("input{index}")),
        None => Cow::Borrowed("input"),
    };
    if fancy_error {
        LatexError::new_err(render_fancy_error(error, &source_name, latex))
    } else {
        let mut err = match index {
            Some(_) => format!("{source_name}:"),
            None => String::new(),
        };
        error.to_message(&mut err, latex);
        LatexError::new_err(err)
    }
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
