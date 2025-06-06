//! math_core
//!
//! Provides a functionality to convert LaTeX math equations to MathML representation.
//! This crate is implemented in pure Rust, so it works for all platforms including WebAssembly.
//!
//! # Supported LaTeX commands
//!
//! - Numbers, e.g. `0`, `3.14`, ...
//! - ASCII and Greek (and more) letters, e.g. `x`, `\alpha`, `\pi`, `\aleph`, ...
//! - Symbols, e.g., `\infty`, `\dagger`, `\angle`, `\Box`, `\partial`, ...
//! - Binary relations, e.g. `=`, `>`, `<`, `\ll`, `:=`, ...
//! - Binary operations, e.g. `+`. `-`, `*`, `/`, `\times`, `\otimes`, ...
//! - Basic LaTeX commands, e.g. `\sqrt`, `\frac`, `\sin`, `\binom`, ...
//! - Parentheses, e.g., `\left\{ .. \middle| .. \right]`, ...
//! - Integrals, e.g., `\int_0^\infty`, `\iint`, `\oint`, ...
//! - Big operators, e.g., `\sum`, `\prod`, `\bigcup_{i = 0}^\infty`, ...
//! - Limits and overset/underset, e.g., `\lim`, `\overset{}{}`, `\overbrace{}{}`, ...
//! - Font styles, e.g. `\mathrm`, `\mathbf`, `\bm`, `\mathit`, `\mathsf`, `\mathscr`, `\mathbb`, `\mathfrak`, `\texttt`.
//!   - MathML lacks calligraphic mathvariant: https://github.com/mathml-refresh/mathml/issues/61
//! - White spaces, e.g., `\!`, `\,`, `\:`, `\;`, `\ `, `\quad`, `\qquad`.
//! - Matrix, e.g. `\begin{matrix}`, `\begin{pmatrix}`, `\begin{bmatrix}`, `\begin{vmatrix}`.
//! - Multi-line equation `\begin{align}` (experimental).
//! - Feynman slash notation: `\slashed{\partial}`.
//!
//! ## Unsupported LaTeX commands
//!
//! - New line `\\`, except for ones in a matrix or align environment.
//! - Alignment `&`, except for ones in a matrix or align environment.
//! - Complicated sub/superscripts (`<mmultiscripts>`).
//!
//!
//! # Usage
//!
//!  Main functions of this crate are  [`latex_to_mathml`](./fn.latex_to_mathml.html) and
//! [`replace`](./fn.replace.html).
//!
//! ```rust
//! use math_core::{Config, Converter, Display};
//!
//! let latex = r#"\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt"#;
//! let mut converter = Converter::new(&Config { pretty: true, ..Default::default() }).unwrap();
//! let mathml = converter.latex_to_mathml(latex, Display::Block).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! For more examples and list of supported LaTeX commands, please check
//! [`examples/equations.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/equations.rs)
//! and [`examples/document.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/document.rs).
//!
mod latex_parser;
mod mathml_renderer;
mod raw_node_slice;

use std::collections::HashMap;

use latex_parser::node_vec_to_node;
pub use latex_parser::{LatexErrKind, LatexError, Token};
use mathml_renderer::arena::Arena;
pub use mathml_renderer::ast::{MathMLEmitter, Node};
use raw_node_slice::RawNodeSlice;

/// display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
}

#[derive(Debug, Default)]
pub struct Config {
    /// If true, the output will be pretty-printed with indentation and newlines.
    pub pretty: bool,
    pub macros: HashMap<String, String>,
}

struct CustomCmds {
    arena: Arena,
    slice: RawNodeSlice,
    map: HashMap<String, (usize, usize)>,
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
        Some(Token::CustomCmd(num_args, node))
    }
}

pub struct Converter {
    pretty: bool,
    /// This is used for numbering equations in the document.
    equation_count: usize,
    custom_cmds: CustomCmds,
}

impl Converter {
    pub fn new(config: &Config) -> Result<Self, LatexError<'_>> {
        Ok(Self {
            pretty: config.pretty,
            equation_count: 0,
            custom_cmds: parse_custom_commands(&config.macros)?,
        })
    }

    /// Convert LaTeX text to MathML.
    ///
    /// The second argument specifies whether it is inline-equation or block-equation.
    ///
    /// ```rust
    /// use math_core::{Config, Converter, Display};
    ///
    /// let latex = r#"(n + 1)! = \Gamma ( n + 1 )"#;
    /// let mut converter = Converter::new(&Config { pretty: true, ..Default::default() }).unwrap();
    /// let mathml = converter.latex_to_mathml(latex, Display::Inline).unwrap();
    /// println!("{}", mathml);
    ///
    /// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
    /// let mathml = converter.latex_to_mathml(latex, Display::Block).unwrap();
    /// println!("{}", mathml);
    /// ```
    ///
    pub fn latex_to_mathml<'config, 'source, 'emitter>(
        &'config mut self,
        latex: &'source str,
        display: Display,
    ) -> Result<String, LatexError<'source>>
    where
        'source: 'emitter,
        'config: 'source,
    {
        let arena = Arena::new();
        let ast = parse(latex, &arena, Some(&self.custom_cmds))?;

        let mut output = MathMLEmitter::new(&mut self.equation_count);
        match display {
            Display::Block => output.push_str("<math display=\"block\">"),
            Display::Inline => output.push_str("<math>"),
        };

        let base_indent = if self.pretty { 1 } else { 0 };
        for node in ast {
            output
                .emit(node, base_indent)
                .map_err(|_| LatexError(0, LatexErrKind::RenderError))?;
        }
        if self.pretty {
            output.push('\n');
        }
        output.push_str("</math>");
        Ok(output.into_inner())
    }

    /// Reset the equation count to zero.
    ///
    /// This should normally be done at the beginning of a new document or section.
    pub fn reset_equation_count(&mut self) {
        self.equation_count = 0;
    }
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
    macros: &'source HashMap<String, String>,
) -> Result<CustomCmds, LatexError<'source>> {
    let arena = Arena::new();
    let mut map = HashMap::with_capacity(macros.len());
    let mut parsed_macros = Vec::with_capacity(macros.len());
    for (name, definition) in macros.iter() {
        let lexer = latex_parser::Lexer::new(definition, true, None);
        let mut p = latex_parser::Parser::new(lexer, &arena);
        let nodes = p.parse()?;
        let num_args = p.l.parse_cmd_args.unwrap_or(0);

        let node_ref = node_vec_to_node(&arena, nodes);
        let index = parsed_macros.len();
        parsed_macros.push(node_ref);
        // TODO: avoid cloning `name` here
        map.insert(name.clone(), (index, num_args));
    }
    let slice = RawNodeSlice::from_slice(arena.push_slice(&parsed_macros));
    Ok(CustomCmds { arena, slice, map })
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::mathml_renderer::ast::MathMLEmitter;
    use crate::{Converter, LatexErrKind, LatexError};

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
            ("d_command", r"\d"),
            ("d_command_nested", r"\mathit{x\d x}"),
            ("RR_command", r"\RR"),
            ("odv", r"\odv{f}{x}"),
            ("xrightarrow", r"\xrightarrow{x}"),
            ("slashed", r"\slashed{\partial}"),
            ("plus_after_equal", r"x = +4"),
            ("equal_after_plus", r"x+ = 4"),
            ("plus_in_braces", r"4{+}4"),
            ("equal_at_group_begin", r"x{=x}"),
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

        let config = crate::Config {
            pretty: true,
            ..Default::default()
        };
        let mut converter = Converter::new(&config).unwrap();
        for (name, problem) in problems.into_iter() {
            converter.reset_equation_count();
            let mathml = converter
                .latex_to_mathml(problem, crate::Display::Inline)
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

        let config = crate::Config {
            macros,
            pretty: true,
        };

        let mut converter = Converter::new(&config).unwrap();

        let latex = r"x = \half";
        let mathml = converter
            .latex_to_mathml(latex, crate::Display::Inline)
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

        let config = crate::Config {
            macros,
            pretty: true,
        };

        let mut converter = Converter::new(&config).unwrap();

        let latex = r"x = \mycmd{3}";
        let mathml = converter
            .latex_to_mathml(latex, crate::Display::Inline)
            .unwrap();

        assert_snapshot!("custom_cmd_one_arg", mathml, latex);
    }
}
