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
//! let converter = LatexToMathML::new(&config).unwrap();
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
mod parse;
mod predefined;
mod specifications;
mod text_parser;
mod token;
mod token_manager;

use rustc_hash::FxHashMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mathml_renderer::{arena::Arena, ast::Node};

pub use self::error::LatexError;
use self::{error::LatexErrKind, lexer::Lexer, parse::Parser, token::Token};

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
/// use rustc_hash::FxHashMap;
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
/// let mut macros: FxHashMap<String, String> = Default::default();
/// macros.insert(
///     "d".to_string(),
///     r"\mathrm{d}".to_string(),
/// );
/// macros.insert(
///     "bb".to_string(),
///     r"\mathbb{#1}".to_string(), // with argument
/// );
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
    /// A map of LaTeX macros; the keys are macro names and the values are their definitions.
    pub macros: FxHashMap<String, String>,
    /// If `true`, include `xmlns="http://www.w3.org/1998/Math/MathML"` in the `<math>` tag.
    pub xml_namespace: bool,
}

#[derive(Debug)]
struct CustomCmds {
    tokens: Box<[Token<'static>]>,
    map: FxHashMap<String, (u8, (usize, usize))>,
    string_literal_store: Box<str>,
}

impl CustomCmds {
    pub fn get_command<'config, 'source>(
        &'config self,
        command: &'source str,
    ) -> Option<Token<'config>>
    where
        'config: 'source,
    {
        let (num_args, slice) = *self.map.get(command)?;
        let tokens = self.tokens.get(slice.0..slice.1)?;
        Some(Token::CustomCmd(num_args, tokens))
    }

    pub fn get_string_literal(&self, start: usize, end: usize) -> Option<&str> {
        self.string_literal_store.get(start..end)
    }
}

/// This struct contains those fields from `MathCoreConfig` that are simple flags.
#[derive(Debug)]
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
#[derive(Debug)]
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
    /// be parsed.
    pub fn new(config: &MathCoreConfig) -> Result<Self, LatexError<'_>> {
        Ok(Self {
            flags: Flags::from(config),
            equation_count: 0,
            custom_cmds: Some(parse_custom_commands(&config.macros)?),
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
    pub fn convert_with_global_counter<'config, 'source>(
        &'config mut self,
        latex: &'source str,
        display: MathDisplay,
    ) -> Result<String, LatexError<'source>>
    where
        'config: 'source,
    {
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
    /// let converter = LatexToMathML::new(&config).unwrap();
    /// let mathml = converter.convert_with_local_counter(latex, MathDisplay::Inline).unwrap();
    /// println!("{}", mathml);
    ///
    /// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
    /// let mathml = converter.convert_with_local_counter(latex, MathDisplay::Block).unwrap();
    /// println!("{}", mathml);
    /// ```
    ///
    #[inline]
    pub fn convert_with_local_counter<'config, 'source>(
        &'config self,
        latex: &'source str,
        display: MathDisplay,
    ) -> Result<String, LatexError<'source>>
    where
        'config: 'source,
    {
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

fn convert<'config, 'source>(
    latex: &'source str,
    display: MathDisplay,
    custom_cmds: Option<&'config CustomCmds>,
    equation_count: &mut u16,
    flags: &Flags,
) -> Result<String, LatexError<'source>>
where
    'config: 'source,
{
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
    output.push_str(">");

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

fn parse<'arena, 'source>(
    latex: &'source str,
    arena: &'arena Arena,
    custom_cmds: Option<&'source CustomCmds>,
    equation_count: &mut u16,
) -> Result<Vec<&'arena Node<'arena>>, LatexError<'source>>
where
    'source: 'arena, // 'source outlives 'arena
{
    let error_slot = std::cell::OnceCell::new();
    let mut string_literal_store = String::new();
    let lexer = Lexer::new(
        latex,
        false,
        custom_cmds,
        &error_slot,
        &mut string_literal_store,
    );
    let mut p = Parser::new(lexer, arena, equation_count).map_err(|e| *e)?;
    let nodes = p.parse().map_err(|e| *e)?;
    Ok(nodes)
}

fn parse_custom_commands<'source>(
    macros: &'source FxHashMap<String, String>,
) -> Result<CustomCmds, LatexError<'source>> {
    let mut map = FxHashMap::with_capacity_and_hasher(macros.len(), Default::default());
    let mut tokens = Vec::new();
    let mut string_literal_store = String::new();
    for (name, definition) in macros.iter() {
        if !is_valid_macro_name(name) {
            return Err(LatexError(0, LatexErrKind::InvalidMacroName(name)));
        }
        let error_slot = std::cell::OnceCell::new();
        let mut lexer: Lexer<'static, '_, '_> = Lexer::new(
            definition,
            true,
            None,
            &error_slot,
            &mut string_literal_store,
        );
        let start = tokens.len();
        loop {
            let tokloc = lexer.next_static_token().map_err(|e| *e)?;
            if matches!(&tokloc.1, Token::Eof) {
                break;
            }
            tokens.push(tokloc.1);
        }
        let end = tokens.len();
        let num_args = lexer.parse_cmd_args().unwrap_or(0);

        // TODO: avoid cloning `name` here
        map.insert(name.clone(), (num_args, (start, end)));
    }
    Ok(CustomCmds {
        tokens: tokens.into_boxed_slice(),
        map,
        string_literal_store: string_literal_store.into_boxed_str(),
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
