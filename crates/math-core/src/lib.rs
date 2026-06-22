//! Convert LaTeX math to MathML Core.
//!
//! For more background on what that means and on what to do with the resulting MathML code,
//! see the repo's README: https://github.com/tmke8/math-core
//!
//! # Usage
//!
//! The main struct of this library is [`LatexToMathML`]. In order to use the library, create an
//! instance of this struct and then call one of the convert functions. The constructor of the
//! struct expects a config object in the form of an instance of [`MathCoreConfig`].
//!
//! Basic use looks like this:
//!
//! ```rust
//! use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
//!
//! let latex = r#"\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt"#;
//! let config = MathCoreConfig::default();
//! let converter = LatexToMathML::new(config).unwrap();
//! let mathml = converter.convert_with_local_state(latex, MathDisplay::Block).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! # Features
//!
//! - `serde`: With this feature, `MathCoreConfig` implements serde's `Deserialize`.
//!
mod atof;
mod character_class;
mod color_defs;
mod commands;
mod environments;
mod error;
mod global_state;
mod html_utils;
mod lexer;
mod parser;
mod predefined;
mod specifications;
mod split_on_ascii;
mod text_parser;
mod token;
mod token_queue;

use rustc_hash::{FxBuildHasher, FxHashMap};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use mathml_renderer::ast::CssClassNames;
use mathml_renderer::{
    arena::Arena,
    ast::{Emitter, Node},
    attribute::Style,
    fmt::new_line_and_indent,
};

pub use self::error::LatexError;
use self::{
    error::LatexErrKind, global_state::GlobalState, lexer::Lexer, parser::Parser, token::Token,
};

/// Display mode for the LaTeX math equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathDisplay {
    /// For inline equations, like those in `$...$` in LaTeX.
    Inline,
    /// For block equations (or "display style" equations), like those in `$$...$$` in LaTeX.
    Block,
}

/// Configuration for pretty-printing the MathML output.
///
/// Pretty-printing means that newlines and indentation is added to the MathML output, to make it
/// easier to read.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum PrettyPrint {
    /// Never pretty print.
    #[default]
    Never,
    /// Always pretty print.
    Always,
    /// Pretty print for block equations only.
    Auto,
}

/// Configuration for using Unicode symbols in the MathML output.
///
/// LaTeX commands like `\coloneqq` can be rendered in MathML either using dedicated Unicode symbols
/// (in this case, `\coloneqq` would be rendered as `≔`) or using a combination of more basic
/// symbols (in this case, `\coloneqq` would be rendered as a combination of `:` and `=`).
/// The former is preferable in terms of semantics but can look a little different from the LaTeX
/// output, while the latter is more faithful to the LaTeX output but can be less semantically
/// clear.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum UnicodeSubstitution {
    /// Never subtitute a set of symbols with their Unicode equivalents.
    Never,
    /// Substitute whenever the LaTeX package `unicode-math` would substitute, which is a good
    /// middle ground between semantics and faithfulness to the LaTeX output.
    #[default]
    Conventional,
    // /// Substitute whenever there is a Unicode equivalent, even if the `unicode-math` package
    // /// does not do so.
    // Aggressive,
}

/// Configuration object for the LaTeX to MathML conversion.
///
/// # Example usage
///
/// ```rust
/// use math_core::{MathCoreConfig, PrettyPrint};
///
/// // Default values
/// let config = MathCoreConfig::default();
///
/// // Specifying pretty-print behavior
/// let config = MathCoreConfig {
///     pretty_print: PrettyPrint::Always,
///     ..Default::default()
///  };
///
/// // Specifying pretty-print behavior and custom macros
/// let macros = vec![
///     ("d".to_string(), r"\mathrm{d}".to_string()),
///     ("bb".to_string(), r"\mathbb{#1}".to_string()), // with argument
/// ];
/// let config = MathCoreConfig {
///     pretty_print: PrettyPrint::Auto,
///     macros,
///     ..Default::default()
/// };
/// ```
///
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default, rename_all = "kebab-case"))]
pub struct MathCoreConfig {
    /// A configuration for pretty-printing the MathML output. See [`PrettyPrint`] for details.
    pub pretty_print: PrettyPrint,
    /// A list of LaTeX macros; each tuple contains (macro_name, macro_definition).
    #[cfg_attr(feature = "serde", serde(with = "tuple_vec_map"))]
    pub macros: Vec<(String, String)>,
    /// If `true`, include `xmlns="http://www.w3.org/1998/Math/MathML"` in the `<math>` tag.
    pub xml_namespace: bool,
    /// If `true`, unknown commands will be rendered as red text in the output, instead of
    /// returning an error.
    pub ignore_unknown_commands: bool,
    /// If `true`, wrap the MathML output in `<semantics>` tags with an
    /// `<annotation encoding="application/x-tex">` child containing the original LaTeX source.
    pub annotation: bool,
    /// If `true`, allow rendering commands that produce MathML Core output that is unreliably
    /// rendered by browsers.
    pub allow_unreliable_rendering: bool,
    /// If not `UnicodeSubstitution::Never`, substitute certain LaTeX commands with their Unicode
    /// equivalents in the MathML output.
    pub unicode_substitution: UnicodeSubstitution,
    /// CSS class names for various elements in the output.
    pub css_classes: CssClassNames,
}

/// A map from custom command names to their number of arguments and the slice of tokens that
/// defines the command. The tokens are stored in a separate vector.
type CustomCmdMap = FxHashMap<String, (u8, (usize, usize))>;

/// Subset of `MathCoreConfig` relevant for the parser.
#[derive(Debug, Default)]
struct ParserConfig {
    custom_cmd_tokens: Vec<Token<'static>>,
    custom_cmd_map: CustomCmdMap,
    ignore_unknown_commands: bool,
    allow_unreliable_rendering: bool,
    unicode_substitution: UnicodeSubstitution,
}

impl ParserConfig {
    pub fn get_command<'config>(&'config self, command: &str) -> Option<Token<'config>> {
        let (num_args, slice) = *self.custom_cmd_map.get(command)?;
        let tokens = self.custom_cmd_tokens.get(slice.0..slice.1)?;
        Some(Token::CustomCmd(num_args, tokens))
    }
}

/// Subset of `MathCoreConfig` relevant for the emitter.
#[derive(Debug, Default)]
struct EmitterConfig {
    pretty_print: PrettyPrint,
    xml_namespace: bool,
    annotation: bool,
    css_classes: CssClassNames,
}

impl From<MathCoreConfig> for EmitterConfig {
    fn from(config: MathCoreConfig) -> Self {
        // FIXME: can we use a macro here to avoid repeating the field names?
        Self {
            pretty_print: config.pretty_print,
            xml_namespace: config.xml_namespace,
            annotation: config.annotation,
            css_classes: config.css_classes,
        }
    }
}

type ParseResult<T> = Result<T, Box<LatexError>>;

/// The error type returned when parsing a custom macro definition fails. Contains the parsing
/// error, the index of the macro definition in the `macros` vector and the macro definition itself.
pub type MacroParseError = (Box<LatexError>, usize, String);

/// A converter that transforms LaTeX math equations into MathML Core.
#[derive(Debug, Default)]
pub struct LatexToMathML {
    emitter_cfg: EmitterConfig,
    state: GlobalState,
    parser_cfg: ParserConfig,
}

impl LatexToMathML {
    /// Create a new `LatexToMathML` converter with the given configuration.
    ///
    /// This function returns an error if the custom macros in the given configuration could not
    /// be parsed. The error contains the parsing error, the macro index and the macro definition
    /// that caused the error.
    pub fn new(mut config: MathCoreConfig) -> Result<Self, MacroParseError> {
        let (custom_cmd_tokens, custom_cmd_map) = parse_custom_commands(
            std::mem::take(&mut config.macros),
            config.unicode_substitution,
        )?;
        let parser_cfg = ParserConfig {
            custom_cmd_tokens,
            custom_cmd_map,
            ignore_unknown_commands: config.ignore_unknown_commands,
            allow_unreliable_rendering: config.allow_unreliable_rendering,
            unicode_substitution: config.unicode_substitution,
        };
        Ok(Self {
            emitter_cfg: EmitterConfig::from(config),
            state: GlobalState::default(),
            parser_cfg,
        })
    }

    /// Convert LaTeX to MathML with a global equation counter.
    ///
    /// For basic usage, see the documentation of [`convert_with_local_state`].
    ///
    /// This conversion function maintains state, in order to count equations correctly across
    /// different calls to this function.
    ///
    /// The counter can be reset with [`reset_global_state`].
    pub fn convert_with_global_state(
        &mut self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError>> {
        convert(
            latex,
            display,
            &self.parser_cfg,
            &mut self.state,
            &self.emitter_cfg,
        )
    }

    /// Convert LaTeX to MathML.
    ///
    /// The second argument specifies whether it is inline-equation or block-equation.
    ///
    /// ```rust
    /// use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
    ///
    /// let latex = r#"(n + 1)! = \Gamma ( n + 1 )"#;
    /// let config = MathCoreConfig::default();
    /// let converter = LatexToMathML::new(config).unwrap();
    /// let mathml = converter.convert_with_local_state(latex, MathDisplay::Inline).unwrap();
    /// println!("{}", mathml);
    ///
    /// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
    /// let mathml = converter.convert_with_local_state(latex, MathDisplay::Block).unwrap();
    /// println!("{}", mathml);
    /// ```
    ///
    pub fn convert_with_local_state(
        &self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError>> {
        let mut state = GlobalState::default();
        convert(
            latex,
            display,
            &self.parser_cfg,
            &mut state,
            &self.emitter_cfg,
        )
    }

    /// Reset the equation counter and the label map.
    ///
    /// This should normally be done at the beginning of a new document or section.
    pub fn reset_global_state(&mut self) {
        self.state.equation_count = 0;
        self.state.label_map.clear();
    }

    /// Convert a collection of LaTeX snippets to MathML.
    ///
    /// This method handles *forward references* correctly, meaning that if an earlier snippet
    /// contains a reference to an equation in a later snippet, the reference will be resolved
    /// correctly. However, in order to achieve this, all snippets need to be parsed first and can
    /// only then be emitted. This means you have to first extract all LaTeX snippets from your
    /// document and then call this method with the whole set.
    pub fn convert_all(
        &self,
        snippets: &[(&str, MathDisplay)],
    ) -> Vec<Result<String, Box<LatexError>>> {
        let mut state = GlobalState::default();
        let arena = Arena::new();
        let ast_vec: Vec<ParseResult<(Vec<&Node<'_>>, &str, MathDisplay)>> = snippets
            .iter()
            .map(|(latex, display)| {
                parse(latex, &arena, &self.parser_cfg, &mut state, *display)
                    .map(|ast| (ast, *latex, *display))
            })
            .collect::<Vec<_>>();
        ast_vec
            .into_iter()
            .map(|ast_result| {
                ast_result.map(|(ast, latex, display)| {
                    emit(
                        ast,
                        latex,
                        display,
                        &state.label_map,
                        &arena,
                        &self.emitter_cfg,
                    )
                })
            })
            .collect()
    }
}

fn convert(
    latex: &str,
    display: MathDisplay,
    parser_cfg: &ParserConfig,
    state: &mut GlobalState,
    flags: &EmitterConfig,
) -> Result<String, Box<LatexError>> {
    let arena = Arena::new();
    let ast = parse(latex, &arena, parser_cfg, state, display)?;
    Ok(emit(ast, latex, display, &state.label_map, &arena, flags))
}

fn emit(
    ast: Vec<&Node>,
    latex: &str,
    display: MathDisplay,
    label_map: &FxHashMap<Box<str>, Box<str>>,
    arena: &Arena,
    flags: &EmitterConfig,
) -> String {
    let mut output = String::new();
    output.push_str("<math");
    if flags.xml_namespace {
        output.push_str(" xmlns=\"http://www.w3.org/1998/Math/MathML\"");
    }
    if matches!(display, MathDisplay::Block) {
        output.push_str(" display=\"block\"");
    }
    output.push('>');

    let pretty_print = matches!(flags.pretty_print, PrettyPrint::Always)
        || (matches!(flags.pretty_print, PrettyPrint::Auto) && display == MathDisplay::Block);

    let base_indent = if pretty_print { 1 } else { 0 };
    if flags.annotation {
        let children_indent = if pretty_print { 2 } else { 0 };
        new_line_and_indent(&mut output, base_indent);
        output.push_str("<semantics>");
        let node = parser::node_vec_to_node(arena, &ast, false);
        let mut emitter = Emitter::new(std::mem::take(&mut output), label_map, &flags.css_classes);
        let _ = emitter.emit(node, children_indent);
        output = emitter.into_string();
        new_line_and_indent(&mut output, children_indent);
        output.push_str("<annotation encoding=\"application/x-tex\">");
        html_utils::escape_html_content(&mut output, latex);
        output.push_str("</annotation>");
        new_line_and_indent(&mut output, base_indent);
        output.push_str("</semantics>");
    } else {
        let mut emitter = Emitter::new(std::mem::take(&mut output), label_map, &flags.css_classes);
        for node in ast {
            // We ignore the result of `emit` here, because the only possible error is a formatting
            // error when writing to the string, but `String`'s `write_str` implementation never
            // returns an error.
            let _ = emitter.emit(node, base_indent);
        }
        output = emitter.into_string();
    }
    if pretty_print {
        output.push('\n');
    }
    output.push_str("</math>");
    output
}

fn parse<'arena>(
    latex: &'arena str,
    arena: &'arena Arena,
    parser_cfg: &'arena ParserConfig,
    state: &mut GlobalState,
    display: MathDisplay,
) -> Result<Vec<&'arena Node<'arena>>, Box<LatexError>> {
    let style = match display {
        MathDisplay::Inline => Style::Text,
        MathDisplay::Block => Style::Display,
    };
    let lexer = Lexer::new(
        latex,
        false,
        Some(parser_cfg),
        parser_cfg.unicode_substitution,
    );
    let mut p = Parser::new(lexer, arena, state, style)?;
    let nodes = p.parse()?;
    Ok(nodes)
}

fn parse_custom_commands(
    macros: Vec<(String, String)>,
    unicode_substitution: UnicodeSubstitution,
) -> Result<(Vec<Token<'static>>, CustomCmdMap), MacroParseError> {
    let mut map = FxHashMap::with_capacity_and_hasher(macros.len(), FxBuildHasher);
    let mut tokens = Vec::new();
    for (idx, (name, definition)) in macros.into_iter().enumerate() {
        if !is_valid_macro_name(name.as_str()) {
            return Err((
                Box::new(LatexError(0..0, LatexErrKind::InvalidMacroName(name))),
                idx,
                definition,
            ));
        }

        // In order to be able to return `definition` in case of an error, we need to ensure
        // that the lexer (which borrows `definition`) is dropped before we return the error.
        // Therefore, we put the whole lexing process into its own block.
        let value = 'value: {
            let mut lexer: Lexer<'static, '_> =
                Lexer::new(definition.as_str(), true, None, unicode_substitution);
            let start = tokens.len();
            loop {
                match lexer.next_token_no_unknown_command() {
                    Ok(tokloc) => {
                        if matches!(tokloc.token(), Token::Eoi) {
                            break;
                        }
                        tokens.push(tokloc.into_token());
                    }
                    Err(err) => {
                        break 'value Err(err);
                    }
                }
            }
            let end = tokens.len();
            let num_args = lexer.parse_cmd_args().unwrap_or(0);
            Ok((num_args, (start, end)))
        };

        match value {
            Err(err) => {
                return Err((err, idx, definition));
            }
            Ok(v) => {
                map.insert(name, v);
            }
        }
    }
    Ok((tokens, map))
}

fn is_valid_macro_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        // If the name contains only one character, any character is valid.
        (Some(_), None) => true,
        // If the name contains more than one character, all characters must be ASCII alphabetic.
        _ => s.bytes().all(|b| b.is_ascii_alphabetic()),
    }
}
