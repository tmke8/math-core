//! Convert LaTeX math to MathML Core.
//!
//! For more background on what that means and on what to do with the resulting MathML code,
//! see the repo's README: <https://github.com/tmke8/math-core>
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
//! let result = converter.convert_with_local_state(latex, MathDisplay::Block).unwrap();
//! println!("{}", result.mathml);
//! ```
//!
//! # Features
//!
//! - `std` (enabled by default): Uses the Rust standard library. Disabling this feature (with
//!   `default-features = false`) makes the crate `no_std`; the `alloc` crate is still required.
//!   Note that disabling `std` also disables some speedups in dependencies (e.g. `memchr` then
//!   can no longer use runtime CPU feature detection).
//! - `serde`: With this feature, `MathCoreConfig` implements serde's `Serialize` and
//!   `Deserialize`.
//! - `ariadne`: Adds `LatexError::to_report()`, which converts an error into an
//!   [`ariadne`](https://docs.rs/ariadne) report for pretty-printing the error together with a
//!   source code snippet. The `ariadne` crate itself requires `std`, so this feature is not
//!   usable on `no_std` targets.
//!
#![cfg_attr(not(any(feature = "std", test)), no_std)]

extern crate alloc;

mod atof;
mod character_class;
mod color_defs;
mod commands;
mod custom_cmds;
mod environments;
mod error;
mod global_state;
mod html_utils;
mod lexer;
mod parser;
mod predefined;
mod specifications;
mod split_on_ascii;
mod string_pool;
mod text_parser;
mod token;
mod token_queue;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use kstring::KString;
use rustc_hash::FxBuildHasher;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Hash map with a fast, non-cryptographic hasher, backed by `hashbrown` so it works in `no_std`.
pub(crate) type FxHashMap<K, V> = hashbrown::HashMap<K, V, FxBuildHasher>;

pub use mathml_renderer::ast::{CssClassNames, IndentKeyword, Indentation, Warnings};
use mathml_renderer::{
    arena::Arena,
    ast::{Emitter, Node},
    attribute::Style,
    fmt::new_line_and_indent,
};

pub use self::error::LatexError;
use self::{
    commands::resolve_builtin_cmd,
    custom_cmds::{CmdSource, CustomCmds, RecordedToken, is_valid_macro_name},
    error::LatexErrKind,
    global_state::GlobalState,
    lexer::{Lexer, LexerOutput},
    parser::Parser,
    token::Token,
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

/// The maximum number of custom command expansions allowed in one snippet.
///
/// Names are resolved when a command is expanded, so a definition may refer to itself, directly or
/// through other definitions, and expanding it would never end. Rather than detecting that, we
/// simply stop after this many expansions, as LaTeX and KaTeX do.
///
/// The default is 1000, which is the same limit that KaTeX uses for its `maxExpand` setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
pub struct MaxExpansions(pub u32);

impl Default for MaxExpansions {
    fn default() -> Self {
        MaxExpansions(1000)
    }
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
    ///
    /// A macro may use another macro of this list, no matter which of the two comes first.
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
    /// If `true`, run the conversion in the global group, which means that commands defined at
    /// the top level of a snippet with `\newcommand` (and related commands) stay defined for the
    /// snippets which come after it.
    ///
    /// If `false` (the default), such definitions are local to the snippet which contains them.
    /// This matches LaTeX, where constructs like `\begin{equation}` and `$$` open a local group,
    /// and it matches the default behavior of KaTeX.
    pub global_group: bool,
    /// If not `UnicodeSubstitution::Never`, substitute certain LaTeX commands with their Unicode
    /// equivalents in the MathML output.
    pub unicode_substitution: UnicodeSubstitution,
    /// CSS class names for various elements in the output.
    pub css_classes: CssClassNames,
    /// The indentation unit used when pretty-printing the MathML output. Either a number of spaces
    /// (e.g. `2`) or the string `"tab"` for a tab character. See [`Indentation`].
    pub indentation: Indentation,
    /// How many custom commands may be expanded in one snippet before the conversion gives up.
    /// See [`MaxExpansions`].
    pub max_expansions: MaxExpansions,
    /// Add this string to the start of every generated `id` attribute.
    /// This is *not* URL escaped. Use the [`percent_encoding`] crate if you need to.
    pub id_prefix: String,
}

/// Subset of `MathCoreConfig` relevant for the parser.
#[derive(Debug, Default)]
struct ParserConfig {
    custom_cmds_from_cfg: CustomCmds,
    ignore_unknown_commands: bool,
    allow_unreliable_rendering: bool,
    global_group: bool,
    unicode_substitution: UnicodeSubstitution,
    max_expansions: MaxExpansions,
}

/// Subset of `MathCoreConfig` relevant for the emitter.
#[derive(Debug, Default)]
struct EmitterConfig {
    pretty_print: PrettyPrint,
    xml_namespace: bool,
    annotation: bool,
    css_classes: CssClassNames,
    indentation: Indentation,
    id_prefix: String,
}

impl From<MathCoreConfig> for EmitterConfig {
    fn from(config: MathCoreConfig) -> Self {
        // FIXME: can we use a macro here to avoid repeating the field names?
        Self {
            pretty_print: config.pretty_print,
            xml_namespace: config.xml_namespace,
            annotation: config.annotation,
            css_classes: config.css_classes,
            indentation: config.indentation,
            id_prefix: config.id_prefix,
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
        let custom_cmds = parse_custom_commands(
            core::mem::take(&mut config.macros),
            config.unicode_substitution,
            config.allow_unreliable_rendering,
        )?;
        let parser_cfg = ParserConfig {
            custom_cmds_from_cfg: custom_cmds,
            ignore_unknown_commands: config.ignore_unknown_commands,
            allow_unreliable_rendering: config.allow_unreliable_rendering,
            global_group: config.global_group,
            unicode_substitution: config.unicode_substitution,
            max_expansions: config.max_expansions,
        };
        Ok(Self {
            emitter_cfg: EmitterConfig::from(config),
            state: GlobalState::default(),
            parser_cfg,
        })
    }

    /// Convert LaTeX to MathML with a global equation counter.
    ///
    /// For basic usage, see the documentation of [`Self::convert_with_local_state`].
    ///
    /// This conversion function maintains state, in order to count equations correctly across
    /// different calls to this function.
    ///
    /// The counter can be reset with [`Self::reset_global_state`].
    pub fn convert_with_global_state(
        &mut self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<ConvertResult, Box<LatexError>> {
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
    /// let result = converter.convert_with_local_state(latex, MathDisplay::Inline).unwrap();
    /// println!("{}", result.mathml);
    ///
    /// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
    /// let result = converter.convert_with_local_state(latex, MathDisplay::Block).unwrap();
    /// println!("{}", result.mathml);
    /// ```
    ///
    pub fn convert_with_local_state(
        &self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<ConvertResult, Box<LatexError>> {
        let mut state = GlobalState::default();
        convert(
            latex,
            display,
            &self.parser_cfg,
            &mut state,
            &self.emitter_cfg,
        )
    }

    /// Reset the equation counter, the label map and the commands defined with `\newcommand`.
    ///
    /// This should normally be done at the beginning of a new document or section.
    pub fn reset_global_state(&mut self) {
        self.state.equation_count = 0;
        self.state.label_map.clear();
        self.state.custom_cmds.clear();
    }

    /// Convert a collection of LaTeX snippets to MathML.
    ///
    /// This method handles *forward references* correctly, meaning that if an earlier snippet
    /// contains a reference to an equation in a later snippet, the reference will be resolved
    /// correctly. However, in order to achieve this, all snippets need to be parsed first and can
    /// only then be emitted. This means you have to first extract all LaTeX snippets from your
    /// document and then call this method with the whole set.
    pub fn convert_all<S: AsRef<str>>(
        &self,
        snippets: &[(S, MathDisplay)],
    ) -> Vec<Result<ConvertResult, Box<LatexError>>> {
        let mut state = GlobalState::default();
        let arena = Arena::new();
        let ast_vec: Vec<ParseResult<(Vec<&Node<'_>>, &str, MathDisplay)>> = snippets
            .iter()
            .map(|(latex, display)| {
                let latex = latex.as_ref();
                parse(latex, &arena, &self.parser_cfg, &mut state, *display)
                    .map(|ast| (ast, latex, *display))
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
) -> Result<ConvertResult, Box<LatexError>> {
    let arena = Arena::new();
    let ast = parse(latex, &arena, parser_cfg, state, display)?;
    Ok(emit(ast, latex, display, &state.label_map, &arena, flags))
}

fn emit(
    ast: Vec<&Node>,
    latex: &str,
    display: MathDisplay,
    label_map: &FxHashMap<KString, KString>,
    arena: &Arena,
    flags: &EmitterConfig,
) -> ConvertResult {
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
    let warnings: Warnings;
    if flags.annotation {
        let children_indent = if pretty_print { 2 } else { 0 };
        new_line_and_indent(&mut output, base_indent, flags.indentation);
        output.push_str("<semantics>");
        let node = parser::node_vec_to_node(arena, &ast, false);
        let mut emitter = Emitter::new(
            core::mem::take(&mut output),
            label_map,
            &flags.css_classes,
            flags.indentation,
            &flags.id_prefix,
        );
        let _ = emitter.emit(node, children_indent);
        warnings = emitter.warnings();
        output = emitter.into_string();
        new_line_and_indent(&mut output, children_indent, flags.indentation);
        output.push_str("<annotation encoding=\"application/x-tex\">");
        html_utils::escape_html_content(&mut output, latex);
        output.push_str("</annotation>");
        new_line_and_indent(&mut output, base_indent, flags.indentation);
        output.push_str("</semantics>");
    } else {
        let mut emitter = Emitter::new(
            core::mem::take(&mut output),
            label_map,
            &flags.css_classes,
            flags.indentation,
            &flags.id_prefix,
        );
        for node in ast {
            // We ignore the result of `emit` here, because the only possible error is a formatting
            // error when writing to the string, but `String`'s `write_str` implementation never
            // returns an error.
            let _ = emitter.emit(node, base_indent);
        }
        warnings = emitter.warnings();
        output = emitter.into_string();
    }
    if pretty_print {
        output.push('\n');
    }
    output.push_str("</math>");
    ConvertResult {
        mathml: output,
        warnings,
    }
}

/// The result of a LaTeX to MathML conversion.
pub struct ConvertResult {
    pub mathml: String,
    pub warnings: Warnings,
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
    let lexer = Lexer::new(latex);
    let mut p = Parser::new(lexer, arena, parser_cfg, state, style)?;
    let nodes = p.parse()?;
    Ok(nodes)
}

/// Read the macros of the configuration into a store of custom commands.
///
/// As in a body recorded from a `\newcommand`, every command is kept as a
/// [`RecordedToken::CommandName`] and only resolved when the macro is used. A macro may
/// therefore refer to a command which is not defined here at all, and in particular to another
/// macro of the configuration, no matter in which order the two are given. Once all macros have
/// been read, every one of those references must point at something, which is what the final
/// check is for; a document definition, which has no such point in time, is only checked when
/// it is used.
fn parse_custom_commands(
    macros: Vec<(String, String)>,
    unicode_substitution: UnicodeSubstitution,
    allow_unreliable_rendering: bool,
) -> Result<CustomCmds, MacroParseError> {
    let mut custom_cmds = CustomCmds::with_capacity(macros.len());
    // The names which have to be defined by the time all macros have been read, together with
    // the macro they appear in and their position within its definition.
    let mut unresolved: Vec<(usize, KString, Range<usize>)> = Vec::new();
    let mut body = Vec::new();
    let parser_cfg = ParserConfig {
        unicode_substitution,
        allow_unreliable_rendering,
        ..Default::default()
    };
    // The definitions are kept around, because the check at the end has to be able to report
    // the one which contains an unresolved name.
    let mut definitions: Vec<String> = Vec::with_capacity(macros.len());
    for (idx, (name, definition)) in macros.into_iter().enumerate() {
        if !is_valid_macro_name(name.as_str()) {
            return Err((
                Box::new(LatexError(0..0, LatexErrKind::InvalidMacroName(name))),
                idx,
                definition,
            ));
        }

        body.clear();
        let mut num_args = 0;
        let mut first_class: Option<character_class::Class> = None;
        let result = 'body: {
            let mut lexer = Lexer::new(definition.as_str());
            loop {
                match lexer.next_token() {
                    Ok(lexer_output) => {
                        let token = match lexer_output {
                            LexerOutput::CommandName(cmd_name, span) => {
                                // We resolve the command here only to know whether it *can* be
                                // resolved and to know its class. The actual resolution is done
                                // when the macro is used.
                                if let Some(resolved) = resolve_builtin_cmd(&parser_cfg, cmd_name) {
                                    if first_class.is_none() {
                                        first_class = resolved.class();
                                    }
                                } else {
                                    unresolved.push((
                                        idx,
                                        KString::from_ref(cmd_name),
                                        span.into(),
                                    ));
                                }
                                body.push(RecordedToken::CommandName(KString::from_ref(cmd_name)));
                                continue;
                            }
                            LexerOutput::Token(tokspan) => tokspan.into_token(),
                        };
                        match token {
                            Token::Eoi => break,
                            Token::CustomCmdArgInput(n) => {
                                if n >= num_args {
                                    num_args = n + 1;
                                }
                                body.push(RecordedToken::Token(Token::CustomCmdArg(n)));
                            }
                            tok => {
                                if first_class.is_none() {
                                    first_class = tok.class();
                                }
                                body.push(RecordedToken::Token(tok))
                            }
                        }
                    }
                    Err(err) => {
                        break 'body Err(err);
                    }
                }
            }
            Ok(())
        };

        if let Err(err) = result {
            return Err((err, idx, definition));
        }
        custom_cmds.insert(name.as_str(), num_args, &body, first_class);
        // The lexer, which borrows the definition, is gone by now.
        definitions.push(definition);
    }
    // Now that all macros are known, every name which none of them defines is an error.
    for (idx, name, span) in unresolved {
        if custom_cmds.get(&name, CmdSource::Config).is_none() {
            let err = Box::new(LatexError(span, LatexErrKind::UnknownCommand(name)));
            return Err((err, idx, definitions.swap_remove(idx)));
        }
    }
    Ok(custom_cmds)
}
