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
mod latex_parser;
mod mathml_renderer;
mod raw_node_slice;

use rustc_hash::FxHashMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use self::latex_parser::{LatexErrKind, NodeRef, Token, node_vec_to_node};
use self::mathml_renderer::arena::{Arena, FrozenArena};
use self::mathml_renderer::ast::{MathMLEmitter, Node};
use self::raw_node_slice::RawNodeSlice;

pub use self::latex_parser::LatexError;

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
}

#[derive(Debug)]
struct CustomCmds {
    arena: FrozenArena,
    slice: RawNodeSlice,
    map: FxHashMap<String, (usize, usize)>,
}

impl CustomCmds {
    pub fn get_command<'config, 'source>(
        &'config self,
        command: &'source str,
    ) -> Option<Token<'source>>
    where
        'config: 'source,
    {
        let (index, num_args) = *self.map.get(command)?;
        let nodes = self.slice.lift(&self.arena)?;
        let node = *nodes.get(index)?;
        Some(Token::CustomCmd(num_args, NodeRef::new(node)))
    }
}

/// A converter that transforms LaTeX math equations into MathML Core.
#[derive(Debug)]
pub struct LatexToMathML {
    pretty_print: PrettyPrint,
    /// This is used for numbering equations in the document.
    equation_count: usize,
    custom_cmds: Option<CustomCmds>,
}

impl LatexToMathML {
    /// Create a new `LatexToMathML` converter with the given configuration.
    ///
    /// This function returns an error if the custom macros in the given configuration could not
    /// be parsed.
    pub fn new(config: &MathCoreConfig) -> Result<Self, LatexError<'_>> {
        Ok(Self {
            pretty_print: config.pretty_print,
            equation_count: 0,
            custom_cmds: Some(parse_custom_commands(&config.macros)?),
        })
    }

    /// Create a new `LatexToMathML` converter with default settings.
    pub const fn const_default() -> Self {
        Self {
            pretty_print: PrettyPrint::Never,
            equation_count: 0,
            custom_cmds: None,
        }
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
            self.pretty_print,
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
            self.pretty_print,
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
    equation_count: &mut usize,
    pretty_print: PrettyPrint,
) -> Result<String, LatexError<'source>>
where
    'config: 'source,
{
    let arena = Arena::new();
    let ast = parse(latex, &arena, custom_cmds)?;

    let mut output = MathMLEmitter::new(equation_count);
    match display {
        MathDisplay::Block => output.push_str("<math display=\"block\">"),
        MathDisplay::Inline => output.push_str("<math>"),
    };

    let pretty_print = matches!(pretty_print, PrettyPrint::Always)
        || (matches!(pretty_print, PrettyPrint::Auto) && display == MathDisplay::Block);

    let base_indent = if pretty_print { 1 } else { 0 };
    for node in ast {
        output
            .emit(node, base_indent)
            .map_err(|_| LatexError(0, LatexErrKind::RenderError))?;
    }
    if pretty_print {
        output.push('\n');
    }
    output.push_str("</math>");
    Ok(output.into_inner())
}

fn parse<'config, 'arena, 'source>(
    latex: &'source str,
    arena: &'arena Arena,
    custom_cmds: Option<&'config CustomCmds>,
) -> Result<Vec<&'arena mathml_renderer::ast::Node<'arena>>, LatexError<'source>>
where
    'source: 'arena,  // 'source outlives 'arena
    'config: 'source, // 'config outlives 'source
{
    let lexer = latex_parser::Lexer::new(latex, false, custom_cmds);
    let mut p = latex_parser::Parser::new(lexer, arena);
    let nodes = p.parse()?;
    Ok(nodes)
}

fn parse_custom_commands<'source>(
    macros: &'source FxHashMap<String, String>,
) -> Result<CustomCmds, LatexError<'source>> {
    let arena = Arena::new();
    let mut map = FxHashMap::with_capacity_and_hasher(macros.len(), Default::default());
    let mut parsed_macros = Vec::with_capacity(macros.len());
    for (name, definition) in macros.iter() {
        if !is_valid_macro_name(name) {
            return Err(LatexError(0, LatexErrKind::InvalidMacroName(&name)));
        }
        let lexer = latex_parser::Lexer::new(definition, true, None);
        let mut p = latex_parser::Parser::new(lexer, &arena);
        let nodes = p.parse()?;
        let num_args = p.l.parse_cmd_args.unwrap_or(0);

        let node_ref = node_vec_to_node(&arena, nodes, true);
        let index = parsed_macros.len();
        parsed_macros.push(node_ref);
        // TODO: avoid cloning `name` here
        map.insert(name.clone(), (index, num_args));
    }
    let slice = RawNodeSlice::from_slice(arena.push_slice(&parsed_macros));
    Ok(CustomCmds {
        arena: arena.freeze(),
        slice,
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

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::mathml_renderer::ast::MathMLEmitter;
    use crate::{LatexErrKind, LatexError, LatexToMathML};

    use super::{Arena, parse};

    fn convert_content(latex: &str) -> Result<String, LatexError> {
        let arena = Arena::new();
        let nodes = parse(latex, &arena, None)?;
        let mut equation_count = 0;
        let mut emitter = MathMLEmitter::new(&mut equation_count);
        for node in nodes.iter() {
            emitter
                .emit(node, 0)
                .map_err(|_| LatexError(0, LatexErrKind::RenderError))?;
        }
        Ok(emitter.into_inner())
    }

    #[test]
    fn full_tests() {
        let problems = [
            ("empty", r""),
            ("only_whitespace", r"  "),
            ("starts_with_whitespace", r"  x  "),
            ("text", r"\text{hi}xx"),
            ("text_multi_space", r"\text{x   y}"),
            ("text_no_braces", r"\text x"),
            ("text_no_braces_space_after", r"\text x y"),
            ("text_no_braces_more_space", r"\text    xx"),
            ("text_then_space", r"\text{x}~y"),
            ("text_nested", r"\text{ \text{a}}"),
            ("text_rq", r"\text{\rq}"),
            (
                "text_diacritics",
                r#"\text{\'{a} \~{a} \.{a} \H{a} \`{a} \={a} \"{a} \v{a} \^{a} \u{a} \r{a} \c{c}}"#,
            ),
            ("text_with_escape_brace", r"\text{a\}b}"),
            ("text_with_weird_o", r"\text{x\o y}"),
            ("text_with_group", r"\text{x{y}z{}p{}}"),
            ("text_with_special_symbols", r"\text{':,=-}"),
            ("textbackslash", r"\text{\textbackslash}"),
            ("textit", r"\textit{x}"),
            ("textbf", r"\textbf{x}"),
            ("textbf_with_digit", r"\textbf{1234}"),
            ("textbf_with_digit_dot", r"\textbf{1234.}"),
            ("textbf_with_digit_decimal", r"\textbf{1234.5}"),
            ("texttt", r"\texttt{x}"),
            ("mathtt", r"\mathtt{x}"),
            ("mathtt_with_digit", r"\mathtt2"),
            ("mathbf_with_digit", r"\mathbf{1234}"),
            ("mathbf_with_digit_dot", r"\mathbf{1234.}"),
            ("mathbf_with_digit_decimal", r"\mathbf{1234.5}"),
            ("integer", r"0"),
            ("rational_number", r"3.14"),
            ("long_number", r"3,453,435.3453"),
            ("number_with_dot", r"4.x"),
            ("long_sub_super", r"x_{92}^{31415}"),
            ("single_variable", r"x"),
            ("greek_letter", r"\alpha"),
            ("greek_letters", r"\phi/\varphi"),
            (
                "greek_letter_tf",
                r"\Gamma\varGamma\boldsymbol{\Gamma\varGamma}",
            ),
            ("greek_letter_boldsymbol", r"\boldsymbol{\alpha}"),
            ("simple_expression", r"x = 3+\alpha"),
            ("sine_function", r"\sin x"),
            ("sine_function_parens", r"\sin(x)"),
            ("sine_function_sqbrackets", r"\sin[x]"),
            ("sine_function_brackets", r"\sin\{x\}"),
            ("sine_sine", r"\sin\sin x"),
            ("sine_at_group_start", r"x{\sin x}"),
            ("sine_at_group_end", r"{x\sin}x"),
            ("sine_at_left", r"x\left(\sin x\right)"),
            ("sine_at_right", r"x\left(x\sin \right)x"),
            ("square_root", r"\sqrt 2"),
            ("square_root_without_space", r"\sqrt12"),
            ("square_root_with_space", r"\sqrt 12"),
            ("complex_square_root", r"\sqrt{x+2}"),
            ("cube_root", r"\sqrt[3]{x}"),
            ("simple_fraction", r"\frac{1}{2}"),
            ("fraction_without_space", r"\frac12"),
            ("fraction_with_space", r"\frac 12"),
            ("slightly_more_complex_fraction", r"\frac{12}{5}"),
            ("superscript", r"x^2"),
            ("sub_superscript", r"x^2_3"),
            ("super_subscript", r"x_3^2"),
            ("double_subscript", r"g_{\mu\nu}"),
            ("simple_accent", r"\dot{x}"),
            ("operator_name", r"\operatorname{sn} x"),
            ("operator_name_with_spaces", r"\operatorname{ hel lo }"),
            ("operator_name_with_single_char", r"\operatorname{a}"),
            ("operator_name_with_space_cmd", r"\operatorname{arg\,max}"),
            ("simple_binomial_coefficient", r"\binom12"),
            ("stretchy_parentheses", r"\left( x \right)"),
            ("stretchy_one-sided_parenthesis", r"\left( x \right."),
            ("simple_integral", r"\int dx"),
            ("contour_integral", r"\oint_C dz"),
            ("simple_overset", r"\overset{n}{X}"),
            ("integral_with_bounds", r"\int_0^1 dx"),
            ("integral_with_lower_bound", r"\int_0 dx"),
            ("integral_with_upper_bound", r"\int^1 dx"),
            ("integral_with_reversed_bounds", r"\int^1_0 dx"),
            ("integral_with_complex_bound", r"\int_{0+1}^\infty"),
            ("integral_with_limits", r"\int\limits_0^1 dx"),
            ("integral_with_lower_limit", r"\int\limits_0 dx"),
            ("integral_with_upper_limit", r"\int\limits^1 dx"),
            ("integral_with_reversed_limits", r"\int\limits^1_0 dx"),
            ("integral_pointless_limits", r"\int\limits dx"),
            ("max_with_limits", r"\max\limits_x"),
            ("bold_font", r"\bm{x}"),
            ("black_board_font", r"\mathbb{R}"),
            ("sum_with_special_symbol", r"\sum_{i = 0}^∞ i"),
            ("sum_with_limit", r"\sum\limits_{i=1}^N"),
            ("sum_pointless_limits", r"\sum\limits n"),
            ("product", r"\prod_n n"),
            ("underscore", r"x\ y"),
            ("stretchy_brace", r"\left\{ x  ( x + 2 ) \right\}"),
            ("stretchy_bracket", r"\left[ x  ( x + 2 ) \right]"),
            ("matrix", r"\begin{pmatrix} x \\ y \end{pmatrix}"),
            (
                "align",
                r#"\begin{align} f ( x ) &= x^2 + 2 x + 1 \\ &= ( x + 1 )^2\end{align}"#,
            ),
            ("align_star", r#"\begin{align*}x&=1\\y=2\end{align*}"#),
            (
                "text_transforms",
                r#"{fi}\ \mathit{fi}\ \mathrm{fi}\ \texttt{fi}"#,
            ),
            ("colon_fusion", r"a := 2 \land b :\equiv 3"),
            ("double_colon", r"a :: b"),
            ("colon_first_group", r"x{:x}"),
            ("colon_approx", r"x:\approx 2"),
            ("colon_sqrt", r"\sqrt :"),
            (
                "cases",
                r"f(x):=\begin{cases}0 &\text{if } x\geq 0\\1 &\text{otherwise.}\end{cases}",
            ),
            ("mathstrut", r"\mathstrut"),
            ("greater_than", r"x > y"),
            ("text_transform_sup", r"\mathbb{N} \cup \mathbb{N}^+"),
            ("overbrace", r"\overbrace{a+b+c}^{d}"),
            ("underbrace", r"\underbrace{a+b+c}_{d}"),
            ("prod", r"\prod_i \prod^n \prod^n_i \prod_i^n"),
            (
                "scriptstyle",
                r"\sum_{\genfrac{}{}{0pt}{}{\scriptstyle 0 \le i \le m}{\scriptstyle 0 < j < n}} P(i, j)",
            ),
            ("genfrac", r"\genfrac(]{0pt}{2}{a+b}{c+d}"),
            ("genfrac_1pt", r"\genfrac(]{1pt}{2}{a+b}{c+d}"),
            (
                "genfrac_1pt_with_space",
                r"\genfrac(]{  1pt     }{2}{a+b}{c+d}",
            ),
            ("genfrac_0.4pt", r"\genfrac(]{0.4pt}{2}{a+b}{c+d}"),
            ("genfrac_0.4ex", r"\genfrac(]{0.4ex}{2}{a+b}{c+d}"),
            ("genfrac_4em", r"\genfrac(]{4em}{2}{a+b}{c+d}"),
            ("not_subset", r"\not\subset"),
            ("not_less_than", r"\not\lt"),
            ("not_less_than_symbol", r"\not< x"),
            ("mathrm_with_superscript", r"\mathrm{x}^2"),
            ("mathrm_with_sin", r"\mathrm{x\sin}"),
            ("mathrm_with_sin2", r"\mathrm{\sin x}"),
            ("mathrm_no_brackets", r"\mathrm x"),
            ("mathit_no_brackets", r"\mathit x"),
            ("mathbb_no_brackets", r"\mathbb N"),
            ("mathit_of_max", r"\mathit{ab \max \alpha\beta}"),
            ("mathit_of_operatorname", r"\mathit{a\operatorname{bc}d}"),
            ("nested_transform", r"\mathit{\mathbf{a}b}"),
            ("mathrm_nested", r"\mathit{\mathrm{a}b}"),
            ("mathrm_nested2", r"\mathrm{\mathit{a}b}"),
            ("mathrm_nested3", r"\mathrm{ab\mathit{cd}ef}"),
            ("mathrm_nested4", r"\mathit{\mathrm{a}}"),
            ("mathrm_multiletter", r"\mathrm{abc}"),
            (
                "complicated_operatorname",
                r"\operatorname {{\pi} o \Angstrom a}",
            ),
            ("operatorname_with_other_operator", r"x\operatorname{\max}"),
            (
                "continued_fraction",
                r"a_0 + \cfrac{1}{a_1 + \cfrac{1}{a_2 + \cfrac{1}{a_3 + \cfrac{1}{a_4}}}}",
            ),
            ("standalone_underscore", "_2F_3"),
            ("really_standalone_underscore", "_2"),
            ("standalone_superscript", "^2F_3"),
            ("really_standalone_superscript", "^2"),
            ("prime", r"f'"),
            ("double_prime", r"f''"),
            ("triple_prime", r"f'''"),
            ("quadruple_prime", r"f''''"),
            ("quintuple_prime", r"f'''''"),
            ("prime_alone", "'"),
            ("prime_and_super", r"f'^2"),
            ("sub_prime_super", r"f_3'^2"),
            ("double_prime_and_super", r"f''^2"),
            ("double_prime_and_super_sub", r"f''^2_3"),
            ("double_prime_and_sub_super", r"f''_3^2"),
            ("sum_prime", r"\sum'"),
            ("int_prime", r"\int'"),
            ("vec_prime", r"\vec{x}'"),
            ("overset_spacing", r"x\overset{!}{=}"),
            ("overset_with_prime", r"\overset{!}{=}'"),
            ("overset_prime", r"\overset{'}{=}"),
            ("overset_plus", r"\overset{!}{+}"),
            ("int_limit_prime", r"\int\limits'"),
            ("prime_command", r"f^\prime"),
            ("prime_command_braces", r"f^{\prime}"),
            ("transform_group", r"\mathit{a{bc}d}"),
            ("nabla_in_mathbf", r"\mathbf{\nabla} + \nabla"),
            ("mathcal_vs_mathscr", r"\mathcal{A}, \mathscr{A}"),
            ("vertical_line", r"P(x|y)"),
            ("mid", r"P(x\mid y)"),
            ("special_symbols", r"\%\$\#"),
            ("lbrack_instead_of_bracket", r"\sqrt\lbrack 4]{2}"),
            ("middle_vert", r"\left(\frac12\middle|\frac12\right)"),
            (
                "middle_uparrow",
                r"\left(\frac12\middle\uparrow\frac12\right)",
            ),
            ("middle_bracket", r"\left(\frac12\middle]\frac12\right)"),
            ("left_right_different_stretch", r"\left/\frac12\right)"),
            ("RR_command", r"\RR"),
            ("odv", r"\odv{f}{x}"),
            ("xrightarrow", r"\xrightarrow{x}"),
            ("slashed", r"\slashed{\partial}"),
            ("plus_after_equal", r"x = +4"),
            ("plus_after_equal_with_space", r"x =\, +4"),
            ("equal_after_plus", r"x+ = 4"),
            ("plus_in_braces", r"4{+}4"),
            ("plus_before_punctuation", r"4+,"),
            ("plus_before_eof", r"4+"),
            ("plus_at_group_end", r"{4+}x"),
            ("equal_at_group_begin", r"x{=x}"),
            ("equal_at_group_end", r"{x=}x"),
            ("equal_at_start_of_pseudo_row", r"x \displaystyle = x"),
            ("equal_at_start_of_pseudo_row_punct", r", \displaystyle = x"),
            ("equal_before_right", r"\left(x=\right)x"),
            ("sqrt_equal_three", r"\sqrt{=3}"),
            ("equal_squared", r"==^2 x"),
            ("equal_vectored", r"=\vec{=}x"),
            ("sqrt_log", r"\sqrt\log"),
            ("sqrt_log_braces", r"\sqrt{\log}"),
            ("root_op", r"\sqrt[+]{x}"),
            ("root_log", r"\sqrt[\log]{x}"),
            ("plus_after_equal_subscript", r"x =_+4"),
            ("plus_after_equal_subscript2", r"x =_2 +4"),
            ("equal_equal", r"4==4"),
            ("subscript_equal_equal", r"x_==4"),
            ("color", r"{\color{Blue}x^2}"),
            ("hspace", r"\hspace{1cm}"),
            ("hspace_whitespace", r"\hspace{  4em }"),
            ("hspace_whitespace_in_between", r"\hspace{  4  em }"),
            ("array_simple", r"\begin{array}{lcr} 0 & 1 & 2 \end{array}"),
            (
                "array_lines",
                r"\begin{array}{ |l| |rc| } 10 & 20 & 30\\ 4 & 5 & 6 \end{array}",
            ),
            (
                "array_many_lines",
                r"\begin{array}{ ||::|l } 10\\ 2 \end{array}",
            ),
            (
                "subarray",
                r"\sum_{\begin{subarray}{c} 0 \le i \le m\\ 0 < j < n \end{subarray}}",
            ),
        ];

        let config = crate::MathCoreConfig {
            pretty_print: crate::PrettyPrint::Always,
            ..Default::default()
        };
        let converter = LatexToMathML::new(&config).unwrap();
        for (name, problem) in problems.into_iter() {
            let mathml = converter
                .convert_with_local_counter(problem, crate::MathDisplay::Inline)
                .expect(format!("failed to convert `{}`", problem).as_str());
            assert_snapshot!(name, &mathml, problem);
        }
    }

    #[test]
    fn error_test() {
        let problems = [
            ("end_without_open", r"\end{matrix}"),
            ("curly_close_without_open", r"}"),
            ("unsupported_command", r"\asdf"),
            (
                "unsupported_environment",
                r"\begin{xmatrix} 1 \end{xmatrix}",
            ),
            ("incorrect_bracket", r"\operatorname[lim}"),
            ("unclosed_bracket", r"\sqrt[lim"),
            ("mismatched_begin_end", r"\begin{matrix} 1 \end{bmatrix}"),
            (
                "spaces_in_env_name",
                r"\begin{  pmatrix   } x \\ y \end{pmatrix}",
            ),
            (
                "incorrect_bracket_in_begin",
                r"\begin{matrix] 1 \end{matrix}",
            ),
            ("incomplete_sup", r"x^"),
            ("invalid_sup", r"x^^"),
            ("invalid_sub_sup", r"x^_"),
            ("double_sub", r"x__3"),
            ("int_double_sub", r"\int__3 x dx"),
            ("unicode_command", r"\éx"),
            ("wrong_opening_paren", r"\begin[matrix} x \end{matrix}"),
            ("unclosed_brace", r"{"),
            ("unclosed_left", r"\left( x"),
            ("unclosed_env", r"\begin{matrix} x"),
            ("unclosed_text", r"\text{hello"),
            ("unexpected_limits", r"\text{hello}\limits_0^1"),
            ("unsupported_not", r"\not\text{hello}"),
            ("text_with_unclosed_group", r"\text{x{}"),
            ("operatorname_with_end", r"\operatorname{\end{matrix}}"),
            ("operatorname_with_begin", r"\operatorname{\begin{matrix}}"),
            ("super_then_prime", "f^2'"),
            ("sub_super_then_prime", "f_5^2'"),
            ("sup_sup", "x^2^3 y"),
            ("sub_sub", "x_2_3 y"),
            ("no_rbrack_instead_of_bracket", r"\sqrt[3\rbrack{1}"),
            ("genfrac_wrong_unit", r"\genfrac(]{1pg}{2}{a+b}{c+d}"),
            ("hspace_empty", r"\hspace{  }"),
            ("hspace_unknown_unit", r"\hspace{2ly}"),
            ("hspace_non_digits", r"\hspace{2b2cm}"),
            ("hspace_non_ascii", r"\hspace{22öm}"),
        ];

        for (name, problem) in problems.into_iter() {
            let LatexError(loc, error) = convert_content(problem).unwrap_err();
            let output = format!("Position: {}\n{:#?}", loc, error);
            assert_snapshot!(name, &output, problem);
        }
    }

    #[test]
    fn test_custom_cmd_zero_arg() {
        let macros = [
            ("half".to_string(), r"\frac{1}{2}".to_string()),
            ("mycmd".to_string(), r"\sqrt{3}".to_string()),
        ]
        .into_iter()
        .collect();

        let config = crate::MathCoreConfig {
            macros,
            pretty_print: crate::PrettyPrint::Always,
        };

        let converter = LatexToMathML::new(&config).unwrap();

        let latex = r"x = \half";
        let mathml = converter
            .convert_with_local_counter(latex, crate::MathDisplay::Inline)
            .unwrap();

        assert_snapshot!("custom_cmd_zero_arg", mathml, latex);
    }
    #[test]
    fn test_custom_cmd_one_arg() {
        let macros = [
            ("half".to_string(), r"\frac{1}{2}".to_string()),
            ("mycmd".to_string(), r"\sqrt{#1}".to_string()),
        ]
        .into_iter()
        .collect();

        let config = crate::MathCoreConfig {
            macros,
            pretty_print: crate::PrettyPrint::Always,
        };

        let converter = LatexToMathML::new(&config).unwrap();

        let latex = r"x = \mycmd{3}";
        let mathml = converter
            .convert_with_local_counter(latex, crate::MathDisplay::Inline)
            .unwrap();

        assert_snapshot!("custom_cmd_one_arg", mathml, latex);
    }
    #[test]
    fn test_custom_cmd_spacing() {
        let macros = [("eq".to_string(), r"=".to_string())].into_iter().collect();

        let config = crate::MathCoreConfig {
            macros,
            pretty_print: crate::PrettyPrint::Always,
        };

        let converter = LatexToMathML::new(&config).unwrap();

        let latex = r"x \eq 3";
        let mathml = converter
            .convert_with_local_counter(latex, crate::MathDisplay::Inline)
            .unwrap();

        assert_snapshot!("custom_cmd_spacing", mathml, latex);
    }
}
