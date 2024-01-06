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
    fmt, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use latex2mmlc::{latex_to_mathml, Display, LatexError};

use clap::Parser;

/// Converts LaTeX formulas to MathML
#[derive(Parser, Debug)]
#[command(version, about = "Converts LaTeX formulas to MathML", long_about = None)]
struct Args {
    /// The HTML file to process
    #[arg(conflicts_with = "formula", value_name = "FILE")]
    file: Option<PathBuf>,

    /// Sets the custom delimiter for LaTeX formulas
    #[arg(short, long, default_value = "$$", conflicts_with = "formula")]
    delimiter: String,

    /// Look recursively for HTML files in the given directory
    #[arg(short, long, conflicts_with = "formula")]
    recursive: bool,

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
    if let Some(ref fpath) = args.file {
        if fpath == &PathBuf::from("-") {
            let input = read_stdin();
            match replace(&input) {
                Ok(mathml) => {
                    println!("{}", mathml);
                }
                Err(e) => exit_latex_error(e),
            };
        } else if args.recursive {
            convert_html_recursive(fpath);
        } else {
            convert_html(fpath);
        };
    } else if let Some(ref formula) = args.formula {
        convert_and_exit(&args, formula);
    } else {
        convert_and_exit(&args, &read_stdin());
    }
}

fn read_stdin() -> String {
    let mut buffer = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        exit_io_error(e);
    }
    buffer
}

fn convert_and_exit(args: &Args, latex: &str) {
    let display = if args.block {
        Display::Block
    } else {
        Display::Inline
    };
    match latex_to_mathml(latex, display, false) {
        Ok(mathml) => println!("{}", mathml),
        Err(e) => exit_latex_error(e),
    }
}

#[derive(Debug)]
enum ConversionError<'a> {
    InvalidNumberOfDollarSigns,
    LatexError(LatexError<'a>, &'a str),
}

impl fmt::Display for ConversionError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConversionError::InvalidNumberOfDollarSigns => {
                write!(f, "Invalid number of dollar signs")
            }
            ConversionError::LatexError(e, input) => write!(f, "Error in '{}':\n{}", input, e),
        }
    }
}

impl std::error::Error for ConversionError<'_> {}

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
fn replace(input: &str) -> Result<String, ConversionError<'_>> {
    let mut output = String::with_capacity(input.len());

    //**** Convert block-math ****//

    // Generate an index list that matches `$$`.
    let slices: Vec<&str> = input.split("$$").collect();
    if slices.len() % 2 == 0 {
        return Err(ConversionError::InvalidNumberOfDollarSigns);
    }

    for (i, outer_slice) in slices.iter().enumerate() {
        if i % 2 == 0 {
            //**** Convert inline-math ****//

            // Generate an index list that matches `$`.
            // (Any `$` that we can still find has to be an inline-math delimiter,
            // because we have already converted the block-math delimiters `$$`.)
            let inner_slices: Vec<&str> = outer_slice.split('$').collect();
            if inner_slices.len() % 2 == 0 {
                return Err(ConversionError::InvalidNumberOfDollarSigns);
            }

            for (i, inner_slice) in inner_slices.iter().enumerate() {
                if i % 2 == 0 {
                    output += inner_slice;
                } else {
                    // convert LaTeX to MathML
                    let mathml = latex_to_mathml(inner_slice, Display::Inline, false)
                        .map_err(|e| ConversionError::LatexError(e, inner_slice))?;
                    output += &mathml;
                }
            }
        } else {
            // convert LaTeX to MathML
            let mathml = latex_to_mathml(outer_slice, Display::Block, false)
                .map_err(|e| ConversionError::LatexError(e, outer_slice))?;
            output += &mathml;
        }
    }

    Ok(output)
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
fn convert_html_recursive<P: AsRef<Path>>(path: P) {
    if path.as_ref().is_dir() {
        let dir = fs::read_dir(path).unwrap_or_else(|e| exit_io_error(e));
        for entry in dir.filter_map(Result::ok) {
            convert_html_recursive(&entry.path())
        }
    } else if path.as_ref().is_file() {
        if let Some(ext) = path.as_ref().extension() {
            if ext == "html" {
                convert_html(&path);
            }
        }
    }
}

fn convert_html<P: AsRef<Path>>(fp: P) {
    let original = fs::read_to_string(&fp).unwrap_or_else(|e| exit_io_error(e));
    let converted = replace(&original).unwrap_or_else(|e| exit_latex_error(e));
    if original != converted {
        let mut fp = fs::File::create(fp).unwrap_or_else(|e| exit_io_error(e));
        fp.write_all(converted.as_bytes())
            .unwrap_or_else(|e| exit_io_error(e));
    }
}

fn exit_latex_error<E: std::error::Error>(e: E) -> ! {
    eprintln!("LaTeX2MathML Error: {}", e);
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
        let mathml = crate::replace(text).unwrap();
        println!("{}", mathml);
    }
}
