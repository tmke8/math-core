//! latex2mmlc_cli
//!
//! For converting a document including LaTeX equations, the function [`replace`](./fn.replace.html)
//! may be useful.
//!
//! ```rust
//! let latex = r#"The error function $\erf ( x )$ is defined by
//! $$\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt .$$"#;
//!
//! let mathml = latex2mmlc_cli::replace(latex).unwrap();
//! println!("{}", mathml);
//! ```
//!
//! If you want to transform the equations in a directory recursively, the function
//! [`convert_html`](./fn.convert_html.html) is useful.
//!
//! ```rust
//! use latex2mmlc_cli::convert_html;
//!
//! convert_html("./target/doc").unwrap();
//! ```

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use clap::Parser;

use math_core::{Config, Display, latex_to_mathml};

use crate::replace::{ConversionError, Replacer};

mod html_entities;
mod replace;

/// Converts LaTeX formulas to MathML
#[derive(Parser, Debug)]
#[command(version, about = "Converts LaTeX formulas to MathML", long_about = None)]
struct Args {
    /// The HTML file to process
    #[arg(conflicts_with = "formula", value_name = "FILE")]
    file: Option<PathBuf>,

    /// Sets the custom delimiter for inline LaTeX formulas
    #[arg(
        long,
        default_value = "$",
        conflicts_with = "formula",
        value_name = "STR"
    )]
    inline_del: String,

    /// Sets the custom delimiter for block LaTeX formulas
    #[arg(
        long,
        default_value = "$$",
        conflicts_with = "formula",
        value_name = "STR"
    )]
    block_del: String,

    /// Sets the custom opening delimiter for inline LaTeX formulas
    #[arg(
        long,
        conflicts_with = "inline_del",
        requires = "inline_close",
        value_name = "STR"
    )]
    inline_open: Option<String>,

    /// Sets the custom closing delimiter for inline LaTeX formulas
    #[arg(
        long,
        conflicts_with = "inline_del",
        requires = "inline_open",
        value_name = "STR"
    )]
    inline_close: Option<String>,

    /// Sets the custom opening delimiter for block LaTeX formulas
    #[arg(
        long,
        conflicts_with = "block_del",
        requires = "block_close",
        value_name = "STR"
    )]
    block_open: Option<String>,

    /// Sets the custom closing delimiter for block LaTeX formulas
    #[arg(
        long,
        conflicts_with = "block_del",
        requires = "block_open",
        value_name = "STR"
    )]
    block_close: Option<String>,

    /// Look recursively for HTML files in the given directory
    #[arg(short, long, conflicts_with = "formula")]
    recursive: bool,

    /// Dry run: convert but don't write anything
    #[arg(long, conflicts_with = "formula")]
    dry_run: bool,

    /// If true, delimiters are ignored that are preceded by a backslash
    #[arg(long, conflicts_with = "formula")]
    ignore_escaped_delim: bool,

    /// Specifies a single LaTeX formula
    #[arg(short, long, conflicts_with = "file")]
    formula: Option<String>,

    /// Sets the display style for the formula to "inline"
    #[arg(short, long, conflicts_with = "file", group = "mode")]
    inline: bool,

    /// Sets the display style for the formula to "block"
    #[arg(short, long, conflicts_with = "file", group = "mode")]
    block: bool,
}

fn main() {
    let args = Args::parse();
    let config = Config::default();
    if let Some(ref fpath) = args.file {
        let inline_delim: (&str, &str) = if let Some(ref open) = args.inline_open {
            (open, &args.inline_close.unwrap())
        } else {
            (&args.inline_del, &args.inline_del)
        };
        let block_delim: (&str, &str) = if let Some(ref open) = args.block_open {
            (open, &args.block_close.unwrap())
        } else {
            (&args.block_del, &args.block_del)
        };
        let mut replacer = Replacer::new(inline_delim, block_delim, args.ignore_escaped_delim);
        if fpath == &PathBuf::from("-") {
            let input = read_stdin();
            match replace(&mut replacer, &input, &config) {
                Ok(mathml) => {
                    println!("{}", mathml);
                }
                Err(e) => exit_latex_error(e, None),
            };
        } else if args.recursive {
            convert_html_recursive(fpath, &mut replacer, &config, args.dry_run);
        } else {
            convert_html(fpath, &mut replacer, &config, args.dry_run);
        };
    } else if let Some(ref formula) = args.formula {
        convert_and_exit(&args, formula, &config);
    } else {
        convert_and_exit(&args, &read_stdin(), &config);
    }
}

fn read_stdin() -> String {
    let mut buffer = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        exit_io_error(e);
    }
    buffer
}

fn convert_and_exit(args: &Args, latex: &str, config: &Config) {
    let display = if args.block {
        Display::Block
    } else {
        Display::Inline
    };
    match latex_to_mathml(latex, display, config) {
        Ok(mathml) => println!("{}", mathml),
        Err(e) => exit_latex_error(e, None),
    }
}

/// Find LaTeX equations and replace them to MathML.
///
/// The delimiters are configured by the `replacer` argument.
///
/// A common configuration is to use `("$", "$")` for inline equations and `("$$", "$$")` for block
/// equations.
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
/// let replacer = math_core::Replacer::new(("$", "$"), ("$$", "$$"));
/// let output = math_core::replace(&replacer, input).unwrap();
/// println!("{}", output);
/// ```
///
fn replace<'source, 'buf>(
    replacer: &'buf mut Replacer,
    input: &'source str,
    config: &Config,
) -> Result<String, ConversionError<'buf>>
where
    'source: 'buf,
{
    replacer.replace(input, |buf, latex, display| {
        let result = latex_to_mathml(latex, display, config)?;
        buf.push_str(result.as_str());
        Ok(())
    })
}

/// Convert all LaTeX expressions for all HTML files in a given directory.
///
/// The argument of this function can be a file name or a directory name.
/// For the latter case, all HTML files in the directory is coneverted.
/// If conversion is failed for a file, then this function does not change
/// the file. The extension of HTML files must be ".html", and `.htm` files
/// are ignored.
///
/// Note that this function uses `math_core::replace`, so the dollar signs
/// are not allowed except for ones enclosing a LaTeX expression.
///
/// # Examples
///
/// This function is meant to replace all LaTeX equations in HTML files
/// generated by `cargo doc`.
///
/// ```rust
/// use math_core::convert_html;
///
/// convert_html("./target/doc").unwrap();
/// ```
///
/// Then all LaTeX equations in HTML files under the directory `./target/doc`
/// will be converted into MathML.
///
fn convert_html_recursive<P: AsRef<Path>>(
    path: P,
    replacer: &mut Replacer,
    config: &Config,
    dry_run: bool,
) {
    if path.as_ref().is_dir() {
        let dir = fs::read_dir(path).unwrap_or_else(|e| exit_io_error(e));
        for entry in dir.filter_map(Result::ok) {
            convert_html_recursive(entry.path(), replacer, config, dry_run)
        }
    } else if path.as_ref().is_file() {
        if let Some(ext) = path.as_ref().extension() {
            if ext == "html" {
                convert_html(&path, replacer, config, dry_run);
            }
        }
    }
}

fn convert_html<P: AsRef<Path>>(fp: P, replacer: &mut Replacer, config: &Config, dry_run: bool) {
    let original = fs::read_to_string(&fp).unwrap_or_else(|e| exit_io_error(e));
    let converted = replace(replacer, &original, config)
        .unwrap_or_else(|e| exit_latex_error(e, Some(fp.as_ref())));
    if !dry_run && original != converted {
        let mut fp = fs::File::create(fp).unwrap_or_else(|e| exit_io_error(e));
        fp.write_all(converted.as_bytes())
            .unwrap_or_else(|e| exit_io_error(e));
    }
}

fn exit_latex_error<E: std::error::Error>(e: E, fp: Option<&Path>) -> ! {
    eprint!("Conversion error");
    if let Some(fp) = fp {
        eprint!(" in '{}'", fp.display());
    }
    eprintln!(": {}", e);
    std::process::exit(2);
}

fn exit_io_error(e: std::io::Error) -> ! {
    eprintln!("IO Error: {}", e);
    std::process::exit(1);
}

#[cfg(test)]
mod tests {

    #[test]
    fn full_test() {
        let text = r#"
Let us consider a rigid sphere (i.e., one having a spherical figure when tested in the stationary system) of radius $R$
which is at rest relative to the system ($K$), and whose centre coincides with the origin of $K$ then the equation of the
surface of this sphere, which is moving with a velocity $v$ relative to $K$, is
$$\xi^2 + \eta^2 + \zeta^2 = R^2$$

At time $t = 0$ the equation is expressed by means of $(x, y, z, t)$ as
$$\frac{ x^2 }{ \left( \sqrt{ 1 - \frac{ v^2 }{ c^2 } } \right)^2 } + y^2 + z^2 = R^2 .$$

A rigid body which has the figure of a sphere when measured in the moving system, has therefore in the moving
condition — when considered from the stationary system, the figure of a rotational ellipsoid with semi-axes
$$R {\sqrt{1-{\frac {v^{2}}{c^{2}}}}}, \ R, \ R .$$
"#;
        let config = math_core::Config::default();
        let mut replacer = crate::Replacer::new(("$", "$"), ("$$", "$$"), false);
        let mathml = crate::replace(&mut replacer, text, &config).unwrap();
        println!("{}", mathml);
    }
}
