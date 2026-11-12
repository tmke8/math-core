extern crate alloc;

#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// SAFETY: This application is single threaded, so using AssumeSingleThreaded is allowed.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
    unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

use js_sys::{Array, Map};
use math_core::{
    CssClassNames, LatexError as CoreLatexError, MathDisplay, MaxExpansions, PrettyPrint,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ConfigParseError {
    message: &'static str,
}

#[wasm_bindgen]
impl ConfigParseError {
    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    pub fn message(&self) -> JsValue {
        JsValue::from_str(self.message)
    }
}

#[wasm_bindgen]
pub struct LatexError {
    message: JsValue,
    label: JsValue,
    pub start: u32,
    pub end: u32,
    /// The index of the snippet that caused the error; only set by `convert_all` and by
    /// `InteractiveDocument`.
    pub index: Option<u32>,
    context: Option<JsValue>,
    report: Option<JsValue>,
}

#[wasm_bindgen]
impl LatexError {
    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    pub fn message(&self) -> JsValue {
        self.message.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "string")]
    pub fn label(&self) -> JsValue {
        self.label.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "string | undefined")]
    pub fn context(&self) -> Option<JsValue> {
        self.context.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "string | undefined")]
    pub fn report(&self) -> Option<JsValue> {
        self.report.clone()
    }
}

#[cfg(feature = "ariadne")]
fn error_report(error: &CoreLatexError, source_name: &str, input: &str) -> Option<JsValue> {
    let report = error.to_report(source_name, false);
    let mut buf = Vec::new();
    report
        .write((source_name, ariadne::Source::from(input)), &mut buf)
        .expect("failed to write report");
    Some(JsValue::from_str(
        &String::from_utf8(buf).expect("report should be valid UTF-8"),
    ))
}

#[cfg(not(feature = "ariadne"))]
fn error_report(_error: &CoreLatexError, _source_name: &str, _input: &str) -> Option<JsValue> {
    None
}

#[wasm_bindgen]
pub struct LatexToMathML {
    inner: math_core::LatexToMathML,
    throw_on_error: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const ITEXT_STYLE: &'static str = r#"
interface MathCoreOptions {
    prettyPrint?: "never" | "always" | "auto";
    macros?: Map<string, string>;
    xmlNamespace?: boolean;
    throwOnError?: boolean;
    ignoreUnknownCommands?: boolean;
    annotation?: boolean;
    allowUnreliableRendering?: boolean;
    globalGroup?: boolean;
    unicodeSubstitution?: "never" | "conventional";
    maxExpansions?: number;
    idPrefix?: string;
    tags?: "none" | "ams" | "all";
}

interface MathSnippet {
    latex: string;
    displayStyle: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "MathSnippet")]
    pub type MathSnippet;

    #[wasm_bindgen(method, getter)]
    fn latex(this: &MathSnippet) -> String;

    #[wasm_bindgen(method, getter)]
    fn displayStyle(this: &MathSnippet) -> bool;

    #[wasm_bindgen(typescript_type = "MathCoreOptions")]
    pub type MathCoreOptions;

    #[wasm_bindgen(method, getter)]
    fn prettyPrint(this: &MathCoreOptions) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    fn macros(this: &MathCoreOptions) -> Option<Map>;

    #[wasm_bindgen(method, getter)]
    fn xmlNamespace(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn throwOnError(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn ignoreUnknownCommands(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn annotation(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn allowUnreliableRendering(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn globalGroup(this: &MathCoreOptions) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn unicodeSubstitution(this: &MathCoreOptions) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    fn maxExpansions(this: &MathCoreOptions) -> Option<u32>;

    #[wasm_bindgen(method, getter)]
    fn idPrefix(this: &MathCoreOptions) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    fn tags(this: &MathCoreOptions) -> Option<String>;
}

/// The options of a `MathCoreOptions` object, in the form the core library wants them.
struct ParsedOptions {
    config: math_core::MathCoreConfig,
    macros: Vec<(String, String)>,
    throw_on_error: bool,
}

impl ParsedOptions {
    fn parse(js_config: &MathCoreOptions) -> Result<Self, JsValue> {
        // This is the poor man's `serde_wasm_bindgen::from_value`.
        let macros = match js_config.macros() {
            Some(macro_map) => {
                let mut macros = Vec::with_capacity(macro_map.size() as usize);
                let macro_iter = macro_map.entries();
                let success = loop {
                    let Ok(entry) = macro_iter.next() else {
                        break false; // Error getting next entry.
                    };
                    if entry.done() {
                        break true; // Exit the loop when there are no more entries.
                    }
                    let Ok(value) = entry.value().dyn_into::<Array>() else {
                        break false; // Error if the value is not an Array.
                    };
                    let Some(key) = value.get(0).as_string() else {
                        break false; // Error if the first element is not a string.
                    };
                    let Some(value) = value.get(1).as_string() else {
                        break false; // Error if the second element is not a string.
                    };
                    macros.push((key, value));
                };
                if success {
                    Some(macros)
                } else {
                    return Err(ConfigParseError {
                        message: "Invalid macros map",
                    }
                    .into());
                }
            }
            None => None,
        };
        let pretty_print = if let Some(pp) = js_config.prettyPrint() {
            match pp.as_str() {
                "always" => Some(PrettyPrint::Always),
                "never" => Some(PrettyPrint::Never),
                "auto" => Some(PrettyPrint::Auto),
                _ => {
                    return Err(ConfigParseError {
                        message: "Invalid value for prettyPrint",
                    }
                    .into());
                }
            }
        } else {
            None
        };
        let xml_namespace = js_config.xmlNamespace().unwrap_or_default();
        let throw_on_error = js_config.throwOnError();
        let ignore_unknown_commands = js_config.ignoreUnknownCommands().unwrap_or_default();
        let annotation = js_config.annotation().unwrap_or_default();
        let allow_unreliable_rendering = js_config.allowUnreliableRendering().unwrap_or_default();
        let global_group = js_config.globalGroup().unwrap_or_default();
        let unicode_substitution = if let Some(us) = js_config.unicodeSubstitution() {
            match us.as_str() {
                "never" => math_core::UnicodeSubstitution::Never,
                "conventional" => math_core::UnicodeSubstitution::Conventional,
                _ => {
                    return Err(ConfigParseError {
                        message: "Invalid value for unicodeSubstitution",
                    }
                    .into());
                }
            }
        } else {
            math_core::UnicodeSubstitution::default()
        };
        let id_prefix = js_config.idPrefix();
        let tags = if let Some(tags) = js_config.tags() {
            match tags.as_str() {
                "none" => math_core::TaggingStrategy::None,
                "ams" => math_core::TaggingStrategy::Ams,
                "all" => math_core::TaggingStrategy::All,
                _ => {
                    return Err(ConfigParseError {
                        message: "Invalid value for tags",
                    }
                    .into());
                }
            }
        } else {
            math_core::TaggingStrategy::default()
        };
        let config = math_core::MathCoreConfig {
            pretty_print: pretty_print.unwrap_or_default(),
            xml_namespace,
            ignore_unknown_commands,
            annotation,
            allow_unreliable_rendering,
            global_group,
            unicode_substitution,
            css_classes: CssClassNames::default(),
            indentation: math_core::Indentation::default(),
            max_expansions: js_config
                .maxExpansions()
                .map_or_else(MaxExpansions::default, MaxExpansions),
            id_prefix: id_prefix.unwrap_or_default(),
            tags,
        };
        Ok(ParsedOptions {
            config,
            macros: macros.unwrap_or_default(),
            throw_on_error: throw_on_error.unwrap_or(true),
        })
    }
}

/// Build the error which is thrown on the JavaScript side for a macro from the options which
/// could not be parsed.
fn macro_error((e, _, context): math_core::MacroParseError) -> LatexError {
    let start = byte_offset_to_utf16_offset(&context, e.0.start) as u32;
    let end = byte_offset_to_utf16_offset(&context, e.0.end) as u32;
    LatexError {
        message: JsValue::from_str(&e.error_message()),
        label: JsValue::from_str(e.label()),
        start,
        end,
        index: None,
        context: Some(JsValue::from_str(&context)),
        report: error_report(&e, "macro", &context),
    }
}

#[wasm_bindgen]
impl LatexToMathML {
    #[wasm_bindgen(constructor)]
    pub fn new(js_config: &MathCoreOptions) -> Result<Self, JsValue> {
        let ParsedOptions {
            config,
            macros,
            throw_on_error,
        } = ParsedOptions::parse(js_config)?;
        let inner = math_core::LatexToMathML::new(config, macros).map_err(macro_error)?;
        Ok(LatexToMathML {
            inner,
            throw_on_error,
        })
    }

    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_local_state(
        &self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self.inner.convert_with_local_state(content, display) {
            Ok(result) => Ok(JsValue::from_str(&result.mathml)),
            Err(e) => {
                if self.throw_on_error {
                    Err(conversion_error(&e, content, None))
                } else {
                    Ok(JsValue::from_str(&e.to_html(content, display, None)))
                }
            }
        }
    }

    #[cfg(feature = "all-conversions")]
    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn convert_with_global_state(
        &mut self,
        content: &str,
        displaystyle: bool,
    ) -> Result<JsValue, LatexError> {
        let display = if displaystyle {
            MathDisplay::Block
        } else {
            MathDisplay::Inline
        };
        match self.inner.convert_with_global_state(content, display) {
            Ok(result) => Ok(JsValue::from_str(&result.mathml)),
            Err(e) => {
                if self.throw_on_error {
                    Err(conversion_error(&e, content, None))
                } else {
                    Ok(JsValue::from_str(&e.to_html(content, display, None)))
                }
            }
        }
    }

    /// Convert a collection of LaTeX snippets to MathML.
    ///
    /// In contrast to the other conversion methods, this one resolves *forward references*
    /// correctly, meaning that a snippet can refer to an equation which is only defined in a
    /// later snippet. This is why all snippets of a document have to be passed in at once.
    ///
    /// The conversion does not touch the global state; it uses a fresh state for the whole batch.
    #[cfg(feature = "all-conversions")]
    #[wasm_bindgen(unchecked_return_type = "string[]")]
    pub fn convert_all(&self, snippets: Vec<MathSnippet>) -> Result<JsValue, LatexError> {
        let inputs: Vec<(String, MathDisplay)> = snippets
            .into_iter()
            .map(|snippet| snippet_input(&snippet))
            .collect();

        let output = self
            .inner
            .convert_all(&inputs)
            .into_iter()
            .enumerate()
            .map(|(index, result)| {
                let (latex, display) = &inputs[index];
                match result {
                    Ok(result) => Ok(JsValue::from_str(&result.mathml)),
                    Err(e) => {
                        if self.throw_on_error {
                            Err(conversion_error(&e, latex, Some(index as u32)))
                        } else {
                            Ok(JsValue::from_str(&e.to_html(latex, *display, None)))
                        }
                    }
                }
            })
            .collect::<Result<Vec<JsValue>, LatexError>>()?;
        Ok(output.into())
    }

    pub fn reset_global_state(&mut self) {
        self.inner.reset_global_state();
    }
}

/// Interface for rendering the math in a document which changes over time.
///
/// The challenge is that the snippets in a document depend on each other: an equation number
/// depends on the numbered equations in front of it, a reference depends on the equation it refers
/// to, and a command defined with `\gdef` is available in the snippets after its definition. The
/// object behind this interace therefore keeps all math snippets, in the order in which they
/// appear, together with their output. Whenever a snippet is edited, `InteractiveDocument converts
/// the snippets which are affected by the edit and returns the indices of those whose output has
/// changed; `get` then returns that output.
///
/// References to equations in later snippets are resolved, as with `convert_all`.
#[cfg(feature = "interactive")]
#[wasm_bindgen]
pub struct InteractiveDocument {
    inner: math_core::InteractiveDocument,
    throw_on_error: bool,
}

#[cfg(feature = "interactive")]
#[wasm_bindgen]
impl InteractiveDocument {
    /// Create an empty document.
    ///
    /// The option `throwOnError` decides what `get` does for a snippet which could not be
    /// converted; editing the document never throws because of such a snippet.
    #[wasm_bindgen(constructor)]
    pub fn new(js_config: &MathCoreOptions) -> Result<Self, JsValue> {
        let ParsedOptions {
            config,
            macros,
            throw_on_error,
        } = ParsedOptions::parse(js_config)?;
        let inner = math_core::InteractiveDocument::new(config, macros).map_err(macro_error)?;
        Ok(InteractiveDocument {
            inner,
            throw_on_error,
        })
    }

    /// The number of snippets in the document.
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Add a snippet in front of the one which is currently at `index`, or at the end of the
    /// document if `index` is the number of snippets.
    ///
    /// Returns the indices of the snippets whose output has changed; see `splice`.
    pub fn insert(
        &mut self,
        index: u32,
        content: String,
        displaystyle: bool,
    ) -> Result<Vec<u32>, JsValue> {
        self.splice_inner(index, 0, vec![(content, display(displaystyle))])
    }

    /// Replace the snippet at `index` with another one.
    ///
    /// Returns the indices of the snippets whose output has changed; see `splice`.
    pub fn replace(
        &mut self,
        index: u32,
        content: String,
        displaystyle: bool,
    ) -> Result<Vec<u32>, JsValue> {
        self.check_index(index)?;
        self.splice_inner(index, 1, vec![(content, display(displaystyle))])
    }

    /// Remove the snippet at `index`.
    ///
    /// Returns the indices of the snippets whose output has changed; see `splice`.
    pub fn remove(&mut self, index: u32) -> Result<Vec<u32>, JsValue> {
        self.check_index(index)?;
        self.splice_inner(index, 1, Vec::new())
    }

    /// Remove `delete_count` snippets, starting with the one at `start`, and put `snippets` in
    /// their place, as `Array.prototype.splice` does.
    ///
    /// When several snippets change at once, a single call is better than a sequence of calls,
    /// because the snippets which depend on the changed ones are then converted only once.
    ///
    /// Returns the indices of the snippets whose output has changed, in ascending order. The
    /// indices are the ones the snippets have *after* the call. The indices always include the new
    /// snippets, and also include those snippets whose output is a different one than before, which
    /// can be snippets on either side of the edit.
    pub fn splice(
        &mut self,
        start: u32,
        delete_count: u32,
        snippets: Vec<MathSnippet>,
    ) -> Result<Vec<u32>, JsValue> {
        let snippets = snippets
            .into_iter()
            .map(|snippet| snippet_input(&snippet))
            .collect();
        self.splice_inner(start, delete_count, snippets)
    }

    /// Remove all snippets.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// The current output of the snippet at `index`.
    ///
    /// For a snippet which could not be converted, this throws a `LatexError`, or, if
    /// `throwOnError` is `false`, returns the error as HTML.
    #[wasm_bindgen(unchecked_return_type = "string")]
    pub fn get(&self, index: u32) -> Result<JsValue, JsValue> {
        let position = index as usize;
        let (Some(output), Some((latex, display))) =
            (self.inner.get(position), self.inner.source(position))
        else {
            return Err(index_error());
        };
        match output {
            Ok(result) => Ok(JsValue::from_str(&result.mathml)),
            Err(e) => {
                if self.throw_on_error {
                    Err(conversion_error(e, latex, Some(index)).into())
                } else {
                    Ok(JsValue::from_str(&e.to_html(latex, display, None)))
                }
            }
        }
    }

    fn check_index(&self, index: u32) -> Result<(), JsValue> {
        if (index as usize) < self.inner.len() {
            Ok(())
        } else {
            Err(index_error())
        }
    }

    fn splice_inner(
        &mut self,
        start: u32,
        delete_count: u32,
        snippets: Vec<(String, MathDisplay)>,
    ) -> Result<Vec<u32>, JsValue> {
        let start = start as usize;
        let end = start.saturating_add(delete_count as usize);
        if end > self.inner.len() {
            return Err(index_error());
        }
        let changed = self.inner.splice(start..end, snippets);
        Ok(changed.into_iter().map(|index| index as u32).collect())
    }
}

#[cfg(feature = "interactive")]
fn index_error() -> JsValue {
    js_sys::RangeError::new("snippet index out of range").into()
}

#[cfg(any(feature = "all-conversions", feature = "interactive"))]
fn display(displaystyle: bool) -> MathDisplay {
    if displaystyle {
        MathDisplay::Block
    } else {
        MathDisplay::Inline
    }
}

/// Read a snippet which was passed in from JavaScript.
#[cfg(any(feature = "all-conversions", feature = "interactive"))]
fn snippet_input(snippet: &MathSnippet) -> (String, MathDisplay) {
    (snippet.latex(), display(snippet.displayStyle()))
}

/// Build the error which is thrown on the JavaScript side for a failed conversion.
///
/// `index` is the position of the failing snippet within a batch, if there is a batch.
fn conversion_error(e: &CoreLatexError, content: &str, index: Option<u32>) -> LatexError {
    let start = byte_offset_to_utf16_offset(content, e.0.start) as u32;
    let end = byte_offset_to_utf16_offset(content, e.0.end) as u32;
    LatexError {
        message: JsValue::from_str(&e.error_message()),
        label: JsValue::from_str(e.label()),
        start,
        end,
        index,
        context: None,
        report: error_report(e, "input", content),
    }
}

/// Converts a byte offset in a UTF-8 string to a UTF-16 code unit offset. This is used for mapping
/// error locations from Rust (which uses UTF-8) to JavaScript (which uses UTF-16).
///
/// If the byte offset is not a valid character boundary, the original byte offset is returned.
fn byte_offset_to_utf16_offset(s: &str, byte_offset: usize) -> usize {
    s.get(..byte_offset)
        .map(|s| s.chars().map(char::len_utf16).sum())
        .unwrap_or(byte_offset)
}
