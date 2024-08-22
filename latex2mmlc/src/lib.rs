//! latex2mmlc
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
//! use latex2mmlc::{latex_to_mathml, Display};
//!
//! let latex = r#"\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt"#;
//! let mathml = latex_to_mathml(latex, Display::Block, true).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! For more examples and list of supported LaTeX commands, please check
//! [`examples/equations.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/equations.rs)
//! and [`examples/document.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/document.rs).
//!
use arena::{Buffer, NodeArena};

pub mod arena;
pub mod ast;
pub mod attribute;
pub(crate) mod commands;
mod error;
pub(crate) mod lexer;
pub(crate) mod ops;
pub(crate) mod parse;
pub mod token;
pub use error::LatexError;
use typed_arena::Arena;

/// display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
}

fn get_nodes<'arena, 'source>(
    latex: &'source str,
    arena: &'arena NodeArena<'arena, 'source>,
) -> Result<(ast::Node<'arena, 'source>, Buffer), error::LatexError<'source>>
where
    'source: 'arena,
{
    // The length of the input is an upper bound for the required length for
    // the string buffer.
    let buffer = Buffer::new(latex.len());
    let l = lexer::Lexer::new(latex);
    let mut p = parse::Parser::new(l, arena, buffer);
    let nodes = p.parse()?;
    Ok((nodes, p.buffer))
}

/// Convert LaTeX text to MathML.
///
/// The second argument specifies whether it is inline-equation or block-equation.
///
/// ```rust
/// use latex2mmlc::{latex_to_mathml, Display};
///
/// let latex = r#"(n + 1)! = \Gamma ( n + 1 )"#;
/// let mathml = latex_to_mathml(latex, Display::Inline, true).unwrap();
/// println!("{}", mathml);
///
/// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
/// let mathml = latex_to_mathml(latex, Display::Block, true).unwrap();
/// println!("{}", mathml);
/// ```
///
pub fn latex_to_mathml(
    latex: &'_ str,
    display: Display,
    pretty: bool,
) -> Result<String, error::LatexError<'_>> {
    let arena = Arena::new();
    let (nodes, b) = get_nodes(latex, &arena)?;

    let mut output = match display {
        Display::Block => "<math display=\"block\">".to_string(),
        Display::Inline => "<math>".to_string(),
    };

    nodes.emit(&mut output, &b, if pretty { 1 } else { 0 });
    if pretty {
        output.push('\n');
    }
    output.push_str("</math>");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::{error, latex_to_mathml};

    use super::{get_nodes, Arena};

    fn convert_content(latex: &str) -> Result<String, error::LatexError> {
        let arena = Arena::new();
        let (nodes, b) = get_nodes(latex, &arena)?;
        Ok(nodes.render(&b))
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
            ("text_with_escape_brace", r"\text{a\}b}"),
            ("text_with_weird_o", r"\text{x\o y}"),
            ("text_with_group", r"\text{x{y}z{}p{}}"),
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
            ("simple_expression", r"x = 3+\alpha"),
            ("sine_function", r"\sin x"),
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
            ("nested_transform", r"\mathit{\mathbf{a}b}"),
            ("mathrm_nested", r"\mathit{\mathrm{a}b}"),
            ("mathrm_nested2", r"\mathrm{\mathit{a}b}"),
            ("mathrm_nested3", r"\mathrm{ab\mathit{cd}ef}"),
            ("mathrm_nested4", r"\mathit{\mathrm{a}}"),
            ("mathrm_multiletter", r"\mathrm{abc}"),
            ("complicated_operatorname", r"\operatorname {{\pi} o \o a}"),
            (
                "continued_fraction",
                r"a_0 + \cfrac{1}{a_1 + \cfrac{1}{a_2 + \cfrac{1}{a_3 + \cfrac{1}{a_4}}}}",
            ),
            ("standalone_underscore", "_2F_3"),
            ("prime", r"f'"),
            ("double_prime", r"f''"),
            ("triple_prime", r"f'''"),
            ("prime_and_super", r"f'^2"),
            ("double_prime_and_super", r"f''^2"),
            ("double_prime_and_super_sub", r"f''^2_3"),
            ("double_prime_and_sub_super", r"f''_3^2"),
            ("sum_prime", r"\sum'"),
            ("int_prime", r"\int'"),
            ("int_limit_prime", r"\int\limits'"),
            ("prime_command", r"f^\prime"),
            ("prime_command_braces", r"f^{\prime}"),
            ("transform_group", r"\mathit{a{bc}d}"),
            ("nabla_in_mathbf", r"\mathbf{\nabla} + \nabla"),
            ("vertical_line", r"P(x|y)"),
            ("mid", r"P(x\mid y)"),
            ("special_symbols", r"\%\$\#"),
        ];

        for (name, problem) in problems.into_iter() {
            let mathml = latex_to_mathml(problem, crate::Display::Inline, true)
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
            ("operatorname_with_other_operator", r"\operatorname{\max}"),
            ("text_with_unclosed_group", r"\text{x{}"),
            ("super_then_prime", "f^2'"),
            ("sup_sup", "x^2^3 y"),
            ("sub_sub", "x_2_3 y"),
        ];

        for (name, problem) in problems.into_iter() {
            let error = format!("{:#?}", convert_content(problem).unwrap_err());
            assert_snapshot!(name, &error, problem);
        }
    }
}
