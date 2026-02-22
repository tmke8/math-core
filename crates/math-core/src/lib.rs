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
//! let mathml = converter.convert_with_local_counter(latex, MathDisplay::Block).unwrap();
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
mod html_utils;
mod lexer;
mod parser;
mod predefined;
mod specifications;
mod text_parser;
mod token;
mod token_queue;

use rustc_hash::FxHashMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mathml_renderer::{arena::Arena, ast::Node, fmt::new_line_and_indent};

pub use self::error::LatexError;
use self::{error::LatexErrKind, lexer::Lexer, parser::Parser, token::Token};

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
}

#[derive(Debug, Default)]
struct CommandConfig {
    custom_cmd_tokens: Vec<Token<'static>>,
    custom_cmd_map: FxHashMap<String, (u8, (usize, usize))>,
    ignore_unknown_commands: bool,
}

impl CommandConfig {
    pub fn get_command<'config>(&'config self, command: &str) -> Option<Token<'config>> {
        let (num_args, slice) = *self.custom_cmd_map.get(command)?;
        let tokens = self.custom_cmd_tokens.get(slice.0..slice.1)?;
        Some(Token::CustomCmd(num_args, tokens))
    }
}

/// This struct contains those fields from `MathCoreConfig` that are simple flags.
#[derive(Debug, Default)]
struct Flags {
    pretty_print: PrettyPrint,
    xml_namespace: bool,
    annotation: bool,
}

impl From<&MathCoreConfig> for Flags {
    fn from(config: &MathCoreConfig) -> Self {
        // TODO: can we use a macro here to avoid repeating the field names?
        Self {
            pretty_print: config.pretty_print,
            xml_namespace: config.xml_namespace,
            annotation: config.annotation,
        }
    }
}

/// A converter that transforms LaTeX math equations into MathML Core.
#[derive(Debug, Default)]
pub struct LatexToMathML {
    flags: Flags,
    /// This is used for numbering equations in the document.
    equation_count: u16,
    cmd_cfg: Option<CommandConfig>,
}

impl LatexToMathML {
    /// Create a new `LatexToMathML` converter with the given configuration.
    ///
    /// This function returns an error if the custom macros in the given configuration could not
    /// be parsed. The error contains the parsing error, the macro index and the macro definition
    /// that caused the error.
    pub fn new(config: MathCoreConfig) -> Result<Self, (Box<LatexError>, usize, String)> {
        Ok(Self {
            flags: Flags::from(&config),
            equation_count: 0,
            cmd_cfg: Some(parse_custom_commands(
                config.macros,
                config.ignore_unknown_commands,
            )?),
        })
    }

    /// Convert LaTeX text to MathML with a global equation counter.
    ///
    /// For basic usage, see the documentation of [`convert_with_local_counter`].
    ///
    /// This conversion function maintains state, in order to count equations correctly across
    /// different calls to this function.
    ///
    /// The counter can be reset with [`reset_global_counter`].
    pub fn convert_with_global_counter(
        &mut self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError>> {
        convert(
            latex,
            display,
            self.cmd_cfg.as_ref(),
            &mut self.equation_count,
            &self.flags,
        )
    }

    /// Convert LaTeX text to MathML.
    ///
    /// The second argument specifies whether it is inline-equation or block-equation.
    ///
    /// ```rust
    /// use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
    ///
    /// let latex = r#"(n + 1)! = \Gamma ( n + 1 )"#;
    /// let config = MathCoreConfig::default();
    /// let converter = LatexToMathML::new(config).unwrap();
    /// let mathml = converter.convert_with_local_counter(latex, MathDisplay::Inline).unwrap();
    /// println!("{}", mathml);
    ///
    /// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
    /// let mathml = converter.convert_with_local_counter(latex, MathDisplay::Block).unwrap();
    /// println!("{}", mathml);
    /// ```
    ///
    #[inline]
    pub fn convert_with_local_counter(
        &self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError>> {
        let mut equation_count = 0;
        convert(
            latex,
            display,
            self.cmd_cfg.as_ref(),
            &mut equation_count,
            &self.flags,
        )
    }

    /// Reset the equation counter to zero.
    ///
    /// This should normally be done at the beginning of a new document or section.
    pub fn reset_global_counter(&mut self) {
        self.equation_count = 0;
    }
}

fn convert(
    latex: &str,
    display: MathDisplay,
    cmd_cfg: Option<&CommandConfig>,
    equation_count: &mut u16,
    flags: &Flags,
) -> Result<String, Box<LatexError>> {
    let arena = Arena::new();
    let ast = parse(latex, &arena, cmd_cfg, equation_count)?;

    let mut output = String::new();
    output.push_str("<math");
    if flags.xml_namespace {
        output.push_str(" xmlns=\"http://www.w3.org/1998/Math/MathML\"");
    }
    if matches!(display, MathDisplay::Block) {
        output.push_str(" display=\"block\"");
    };
    output.push('>');

    let pretty_print = matches!(flags.pretty_print, PrettyPrint::Always)
        || (matches!(flags.pretty_print, PrettyPrint::Auto) && display == MathDisplay::Block);

    let base_indent = if pretty_print { 1 } else { 0 };
    if flags.annotation {
        new_line_and_indent(&mut output, base_indent);
        output.push_str("<semantics>");
        let node = parser::node_vec_to_node(&arena, ast, false);
        let _ = node.emit(&mut output, base_indent + 1);
        new_line_and_indent(&mut output, base_indent + 1);
        output.push_str("<annotation encoding=\"application/x-tex\">");
        html_utils::escape_html_content(&mut output, latex);
        output.push_str("</annotation>");
        new_line_and_indent(&mut output, base_indent);
        output.push_str("</semantics>");
    } else {
        for node in ast {
            // We ignore the result of `emit` here, because the only possible error is a formatting
            // error when writing to the string, and that can only happen if the string's `write_str`
            // implementation returns an error. Since `String`'s `write_str` implementation never
            // returns an error, we can safely ignore the result of `emit`.
            let _ = node.emit(&mut output, base_indent);
        }
    }
    if pretty_print {
        output.push('\n');
    }
    output.push_str("</math>");
    Ok(output)
}

fn parse<'arena, 'source, 'config>(
    latex: &'source str,
    arena: &'arena Arena,
    cmd_cfg: Option<&'config CommandConfig>,
    equation_count: &mut u16,
) -> Result<Vec<&'arena Node<'arena>>, Box<LatexError>>
where
    'config: 'source,
    'source: 'arena,
{
    let lexer = Lexer::new(latex, false, cmd_cfg);
    let mut p = Parser::new(lexer, arena, equation_count)?;
    let nodes = p.parse()?;
    Ok(nodes)
}

fn parse_custom_commands(
    macros: Vec<(String, String)>,
    ignore_unknown_commands: bool,
) -> Result<CommandConfig, (Box<LatexError>, usize, String)> {
    let mut map = FxHashMap::with_capacity_and_hasher(macros.len(), Default::default());
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
            let mut lexer: Lexer<'static, '_> = Lexer::new(definition.as_str(), true, None);
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
        };
    }
    Ok(CommandConfig {
        custom_cmd_tokens: tokens,
        custom_cmd_map: map,
        ignore_unknown_commands,
    })
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
