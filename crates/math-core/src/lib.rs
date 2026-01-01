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
mod token_manager;

use rustc_hash::FxHashMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mathml_renderer::{arena::Arena, ast::Node};

pub use self::error::{LatexErrKind, LatexError};
use self::{lexer::Lexer, parser::Parser, token::Token};

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
}

#[derive(Debug, Default)]
struct CustomCmds {
    tokens: Box<[Token<'static>]>,
    map: FxHashMap<String, (u8, (usize, usize))>,
}

impl CustomCmds {
    pub fn get_command<'config>(&'config self, command: &str) -> Option<Token<'config>> {
        let (num_args, slice) = *self.map.get(command)?;
        let tokens = self.tokens.get(slice.0..slice.1)?;
        Some(Token::CustomCmd(num_args, tokens))
    }
}

/// This struct contains those fields from `MathCoreConfig` that are simple flags.
#[derive(Debug, Default)]
struct Flags {
    pretty_print: PrettyPrint,
    xml_namespace: bool,
}

impl From<&MathCoreConfig> for Flags {
    fn from(config: &MathCoreConfig) -> Self {
        // TODO: can we use a macro here to avoid repeating the field names?
        Self {
            pretty_print: config.pretty_print,
            xml_namespace: config.xml_namespace,
        }
    }
}

/// A converter that transforms LaTeX math equations into MathML Core.
#[derive(Debug, Default)]
pub struct LatexToMathML {
    flags: Flags,
    /// This is used for numbering equations in the document.
    equation_count: u16,
    custom_cmds: Option<CustomCmds>,
}

impl LatexToMathML {
    /// Create a new `LatexToMathML` converter with the given configuration.
    ///
    /// This function returns an error if the custom macros in the given configuration could not
    /// be parsed. The error contains both the parsing error and the macro definition that caused
    /// the error.
    pub fn new(config: MathCoreConfig) -> Result<Self, (Box<LatexError<'static>>, String)> {
        Ok(Self {
            flags: Flags::from(&config),
            equation_count: 0,
            custom_cmds: Some(parse_custom_commands(config.macros)?),
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
    pub fn convert_with_global_counter<'config>(
        &'config mut self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError<'config>>> {
        convert(
            latex,
            display,
            self.custom_cmds.as_ref(),
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
    pub fn convert_with_local_counter<'config>(
        &'config self,
        latex: &str,
        display: MathDisplay,
    ) -> Result<String, Box<LatexError<'config>>> {
        let mut equation_count = 0;
        convert(
            latex,
            display,
            self.custom_cmds.as_ref(),
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

fn convert<'config>(
    latex: &str,
    display: MathDisplay,
    custom_cmds: Option<&'config CustomCmds>,
    equation_count: &mut u16,
    flags: &Flags,
) -> Result<String, Box<LatexError<'config>>> {
    let arena = Arena::new();
    let ast = parse(latex, &arena, custom_cmds, equation_count)?;

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
    for node in ast {
        node.emit(&mut output, base_indent)
            .map_err(|_| LatexError(0, LatexErrKind::RenderError))?;
    }
    if pretty_print {
        output.push('\n');
    }
    output.push_str("</math>");
    Ok(output)
}

fn parse<'arena, 'config>(
    latex: &str,
    arena: &'arena Arena,
    custom_cmds: Option<&'config CustomCmds>,
    equation_count: &mut u16,
) -> Result<Vec<&'arena Node<'arena>>, Box<LatexError<'config>>> {
    let lexer = Lexer::new(latex, false, custom_cmds);
    let mut p = Parser::new(lexer, arena, equation_count)?;
    let nodes = p.parse()?;
    Ok(nodes)
}

fn parse_custom_commands(
    macros: Vec<(String, String)>,
) -> Result<CustomCmds, (Box<LatexError<'static>>, String)> {
    let mut map = FxHashMap::with_capacity_and_hasher(macros.len(), Default::default());
    let mut tokens = Vec::new();
    for (name, definition) in macros {
        if !is_valid_macro_name(name.as_str()) {
            return Err((
                Box::new(LatexError(0, LatexErrKind::InvalidMacroName(name))),
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
                match lexer.next_token() {
                    Ok(tokloc) => {
                        if matches!(tokloc.token(), Token::Eof) {
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
                return Err((err, definition));
            }
            Ok(v) => {
                map.insert(name, v);
            }
        };
    }
    Ok(CustomCmds {
        tokens: tokens.into_boxed_slice(),
        map,
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
