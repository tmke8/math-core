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
        let result = if fpath == &PathBuf::from("-") {
            match replace(&read_stdin()) {
                Ok(mathml) => {
                    println!("{}", mathml);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            if args.recursive {
                convert_html_recursive(fpath)
            } else {
                convert_html(fpath)
            }
        };
        if let Err(e) = result {
            eprintln!("LaTeX2MathML Error: {}", e);
            std::process::exit(2);
        }
    } else if let Some(ref formula) = args.formula {
        convert_and_exit(&args, formula);
    } else {
        convert_and_exit(&args, &read_stdin());
    }
}

fn read_stdin() -> String {
    let mut buffer = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        eprintln!("IO Error: {}", e);
        std::process::exit(1);
    }
    buffer
}

fn convert_and_exit(args: &Args, latex: &str) {
    let display = if args.block {
        Display::Block
    } else {
        Display::Inline
    };
    match latex_to_mathml(latex, display) {
        Ok(mathml) => println!("{}", mathml),
        Err(e) => {
            eprintln!("LaTeX2MathML Error: {}", e);
            std::process::exit(2);
        }
    }
}

#[derive(Debug)]
enum ConversionError {
    InvalidNumberOfDollarSigns,
    IOError(std::io::Error),
    LatexError(LatexError),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConversionError::InvalidNumberOfDollarSigns => {
                write!(f, "Invalid number of dollar signs")
            }
            ConversionError::LatexError(e) => write!(f, "{}", e),
            ConversionError::IOError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ConversionError {}

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
fn replace(input: &str) -> Result<String, ConversionError> {
    let mut input: Vec<u8> = input.as_bytes().to_owned();

    //**** Convert block-math ****//

    // `$$` に一致するインデックスのリストを生成
    let idx = input
        .windows(2)
        .enumerate()
        .filter_map(|(i, window)| {
            if window == [b'$', b'$'] {
                Some(i)
            } else {
                None
            }
        })
        .collect::<Vec<usize>>();
    if idx.len() % 2 != 0 {
        return Err(ConversionError::InvalidNumberOfDollarSigns);
    }

    if idx.len() > 1 {
        let mut output = Vec::new();
        output.extend_from_slice(&input[0..idx[0]]);
        for i in (0..idx.len() - 1).step_by(2) {
            {
                // convert LaTeX to MathML
                let input = &input[idx[i] + 2..idx[i + 1]];
                let input = unsafe { std::str::from_utf8_unchecked(input) };
                let mathml =
                    latex_to_mathml(input, Display::Block).map_err(ConversionError::LatexError)?;
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
        return Err(ConversionError::InvalidNumberOfDollarSigns);
    }

    if idx.len() > 1 {
        let mut output = Vec::new();
        output.extend_from_slice(&input[0..idx[0]]);
        for i in (0..idx.len() - 1).step_by(2) {
            {
                // convert LaTeX to MathML
                let input = &input[idx[i] + 1..idx[i + 1]];
                let input = unsafe { std::str::from_utf8_unchecked(input) };
                let mathml =
                    latex_to_mathml(input, Display::Inline).map_err(ConversionError::LatexError)?;
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
fn convert_html_recursive<P: AsRef<Path>>(path: P) -> Result<(), ConversionError> {
    if path.as_ref().is_dir() {
        let dir = fs::read_dir(path).map_err(ConversionError::IOError)?;
        for entry in dir.filter_map(Result::ok) {
            convert_html_recursive(&entry.path())?
        }
    } else if path.as_ref().is_file() {
        if let Some(ext) = path.as_ref().extension() {
            if ext == "html" {
                match convert_html(&path) {
                    Ok(_) => (),
                    Err(e) => eprintln!("LaTeX2MathML Error: {}", e),
                }
            }
        }
    }

    Ok(())
}

fn convert_html<P: AsRef<Path>>(fp: P) -> Result<(), ConversionError> {
    let original = fs::read_to_string(&fp).map_err(ConversionError::IOError)?;
    let converted = replace(&original)?;
    if original != converted {
        let mut fp = fs::File::create(fp).map_err(ConversionError::IOError)?;
        fp.write_all(converted.as_bytes())
            .map_err(ConversionError::IOError)?;
    }
    Ok(())
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
