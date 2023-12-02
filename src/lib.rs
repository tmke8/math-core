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
//! let mathml = latex_to_mathml(latex, Display::Block).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! For converting a document including LaTeX equations, the function [`replace`](./fn.replace.html)
//! may be useful.
//!
//! ```rust
//! let latex = r#"The error function $\erf ( x )$ is defined by
//! $$\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt .$$"#;
//!
//! let mathml = latex2mmlc::replace(latex).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! If you want to transform the equations in a directory recursively, the function
//! [`convert_html`](./fn.convert_html.html) is useful.
//!
//! ```rust
//! use latex2mmlc::convert_html;
//!
//! convert_html("./target/doc").unwrap();
//! ```
//!
//! For more examples and list of supported LaTeX commands, please check
//! [`examples/equations.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/equations.rs)
//! and [`examples/document.rs`](https://github.com/osanshouo/latex2mathml/blob/master/examples/document.rs).
//!

pub mod ast;
pub mod attribute;
mod error;
pub(crate) mod lexer;
pub(crate) mod ops;
pub(crate) mod parse;
pub mod token;
pub use error::LatexError;
use std::{fmt, fs, io::Write, path::Path};

/// display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
}

impl fmt::Display for Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Display::Block => write!(f, "block"),
            Display::Inline => write!(f, "inline"),
        }
    }
}

fn convert_content(latex: &str) -> Result<String, error::LatexError> {
    let l = lexer::Lexer::new(latex);
    let mut p = parse::Parser::new(l);
    let nodes = p.parse()?;

    let mathml = nodes
        .iter()
        .map(|node| format!("{}", node))
        .collect::<String>();

    Ok(mathml)
}

/// Convert LaTeX text to MathML.
///
/// The second argument specifies whether it is inline-equation or block-equation.
///
/// ```rust
/// use latex2mmlc::{latex_to_mathml, Display};
///
/// let latex = r#"(n + 1)! = \Gamma ( n + 1 )"#;
/// let mathml = latex_to_mathml(latex, Display::Inline).unwrap();
/// println!("{}", mathml);
///
/// let latex = r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#;
/// let mathml = latex_to_mathml(latex, Display::Block).unwrap();
/// println!("{}", mathml);
/// ```
///
pub fn latex_to_mathml(latex: &str, display: Display) -> Result<String, error::LatexError> {
    let mathml = convert_content(latex)?;

    Ok(format!(
        r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="{}">
{}</math>"#,
        display, mathml
    ))
}

/// Find LaTeX equations and replace them to MathML.
///
/// - inline-math: `$..$`
/// - display-math: `$$..$$`
///
/// Note that dollar signs that do not enclose a LaTeX equation (e.g. `This apple is $3.`) must not appear
/// in the input string. Dollar sings in LaTeX equation (i.e. `\$` command) must also not appear.
/// Please use `&dollar;`, instead of `$`, outside LaTeX equations.
///
/// ```rust
/// let input = r#"$E = m c^2$ is the most famous equation derived by Einstein.
/// In fact, this relation is a spacial case of the equation
/// $$E = \sqrt{ m^2 c^4 + p^2 c^2 } ,$$
/// which describes the relation between energy and momentum."#;
/// let output = latex2mmlc::replace(input).unwrap();
/// println!("{}", output);
/// ```
///
/// `examples/document.rs` gives a sample code using this function.
///
pub fn replace(input: &str) -> Result<String, error::LatexError> {
    let mut input: Vec<u8> = input.as_bytes().to_owned();

    //**** Convert block-math ****//

    // `$$` に一致するインデックスのリストを生成
    let idx = input
        .windows(2)
        .enumerate()
        .filter_map(|(i, window)| {
            if window == &[b'$', b'$'] {
                Some(i)
            } else {
                None
            }
        })
        .collect::<Vec<usize>>();
    if idx.len() % 2 != 0 {
        return Err(LatexError::InvalidNumberOfDollarSigns);
    }

    if idx.len() > 1 {
        let mut output = Vec::new();
        output.extend_from_slice(&input[0..idx[0]]);
        for i in (0..idx.len() - 1).step_by(2) {
            {
                // convert LaTeX to MathML
                let input = &input[idx[i] + 2..idx[i + 1]];
                let input = unsafe { std::str::from_utf8_unchecked(input) };
                let mathml = latex_to_mathml(input, Display::Block)?;
                output.extend_from_slice(mathml.as_bytes());
            }

            if i + 2 < idx.len() {
                output.extend_from_slice(&input[idx[i + 1] + 2..idx[i + 2]]);
            } else {
                output.extend_from_slice(&input[idx.last().unwrap() + 2..]);
            }
        }

        input = output;
    }

    //**** Convert inline-math ****//

    // `$` に一致するインデックスのリストを生成
    let idx = input
        .iter()
        .enumerate()
        .filter_map(|(i, byte)| if byte == &b'$' { Some(i) } else { None })
        .collect::<Vec<usize>>();
    if idx.len() % 2 != 0 {
        return Err(LatexError::InvalidNumberOfDollarSigns);
    }

    if idx.len() > 1 {
        let mut output = Vec::new();
        output.extend_from_slice(&input[0..idx[0]]);
        for i in (0..idx.len() - 1).step_by(2) {
            {
                // convert LaTeX to MathML
                let input = &input[idx[i] + 1..idx[i + 1]];
                let input = unsafe { std::str::from_utf8_unchecked(input) };
                let mathml = latex_to_mathml(input, Display::Inline)?;
                output.extend_from_slice(mathml.as_bytes());
            }

            if i + 2 < idx.len() {
                output.extend_from_slice(&input[idx[i + 1] + 1..idx[i + 2]]);
            } else {
                output.extend_from_slice(&input[idx.last().unwrap() + 1..]);
            }
        }

        input = output;
    }

    unsafe { Ok(String::from_utf8_unchecked(input)) }
}

/// Convert all LaTeX expressions for all HTMLs in a given directory.
///
/// The argument of this function can be a file name or a directory name.
/// For the latter case, all HTML files in the directory is coneverted.
/// If conversion is failed for a file, then this function does not change
/// the file. The extension of HTML files must be ".html", and `.htm` files
/// are ignored.
///
/// Note that this function uses `latex2mmlc::replace`, so the dollar signs
/// are not allowed except for ones enclosing a LaTeX expression.
///
/// # Examples
///
/// This function is meant to replace all LaTeX equations in HTML files
/// generated by `cargo doc`.
///
/// ```rust
/// use latex2mmlc::convert_html;
///
/// convert_html("./target/doc").unwrap();
/// ```
///
/// Then all LaTeX equations in HTML files under the directory `./target/doc`
/// will be converted into MathML.
///
pub fn convert_html<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    if path.as_ref().is_dir() {
        for entry in fs::read_dir(path)?.filter_map(Result::ok) {
            convert_html(&entry.path())?
        }
    } else if path.as_ref().is_file() {
        if let Some(ext) = path.as_ref().extension() {
            if ext == "html" {
                match convert_latex(&path) {
                    Ok(_) => (),
                    Err(e) => eprintln!("LaTeX2MathML Error: {}", e),
                }
            }
        }
    }

    Ok(())
}

fn convert_latex<P: AsRef<Path>>(fp: P) -> Result<(), Box<dyn std::error::Error>> {
    let original = fs::read_to_string(&fp)?;
    let converted = replace(&original)?;
    if &original != &converted {
        let mut fp = fs::File::create(fp)?;
        fp.write_all(converted.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::convert_content;

    #[test]
    fn it_works() {
        let problems = vec![
            ("integer", r"0"),
            ("rational_number", r"3.14"),
            ("single_variable", r"x"),
            ("greek_letter", r"\alpha"),
            ("greek_letters", r"\phi/\varphi"),
            ("simple_expression", r"x = 3+\alpha"),
            ("sine_function", r"\sin x"),
            ("square_root", r"\sqrt 2"),
            ("square_root_without_space", r"\sqrt12"),
            ("complex_square_root", r"\sqrt{x+2}"),
            ("cube_root", r"\sqrt[3]{x}"),
            ("simple_fraction", r"\frac{1}{2}"),
            ("fraction_without_space", r"\frac12"),
            ("slightly_more_complex_fraction", r"\frac{12}{5}"),
            ("superscript", r"x^2"),
            ("double_subscript", r"g_{\mu\nu}"),
            ("simple_accent", r"\dot{x}"),
            ("operator_name", r"\operatorname{sn} x"),
            ("simple_binomial_coefficient", r"\binom12"),
            ("stretchy_parentheses", r"\left( x \right)"),
            ("stretchy_one-sided_parenthesis", r"\left( x \right."),
            ("simple_integral", r"\int dx"),
            ("contour_integral", r"\oint_C dz"),
            ("simple_overset", r"\overset{n}{X}"),
            ("integral_with_limits", r"\int_0^1 dx"),
            ("integral_with_reversed_limits", r"\int^1_0 dx"),
            ("bold_font", r"\bm{x}"),
            ("black_board_font", r"\mathbb{R}"),
            ("sum_with_special_symbol", r"\sum_{i = 0}^∞ i"),
            ("product", r"\prod_n n"),
            ("underscore", r"x\ y"),
            ("stretchy_brace", r"\left\{ x  ( x + 2 ) \right\}"),
            ("prime", r"f'"),
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
                r"f(x):=\begin{cases}0 &\text{if }x\geq 0\\1 &\text{otherwise}\end{cases}",
            ),
        ];

        for (name, problem) in problems.iter() {
            let mathml = convert_content(dbg!(problem)).unwrap();
            assert_snapshot!(*name, &mathml, problem);
        }
    }
}
