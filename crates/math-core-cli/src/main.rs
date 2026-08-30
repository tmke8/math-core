use std::{
    fs,
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use clap::Parser;

use math_core::{LatexError, LatexToMathML, MathDisplay};

mod config_file;
mod html_entities;
mod replace;

use replace::{ConversionError, LineTracker, Replacer, SnippetWarning};

static DEFAULT_CONFIG_FILE: &str = "mathcore.toml";

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

    /// Overwrite the input file in place instead of writing to stdout
    /// (recursive mode always writes in place)
    #[arg(short, long, conflicts_with_all = ["formula", "recursive"])]
    write: bool,

    /// Dry run: convert but don't write anything
    #[arg(long, conflicts_with = "formula")]
    dry_run: bool,

    /// If true, delimiters are ignored that are preceded by a backslash
    #[arg(long, conflicts_with = "formula")]
    ignore_escaped_delim: bool,

    /// If true, the program continues to convert when an error occurs
    #[arg(long, conflicts_with = "formula")]
    continue_on_error: bool,

    /// Specifies a single LaTeX formula
    #[arg(short, long, conflicts_with = "file")]
    formula: Option<String>,

    /// Sets the display style for the formula to "inline"
    #[arg(short, long, conflicts_with = "file", group = "mode")]
    inline: bool,

    /// Sets the display style for the formula to "block"
    #[arg(short, long, conflicts_with = "file", group = "mode")]
    block: bool,

    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    config_file: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    // Determine which config file to use
    let config_path = args
        .config_file
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE));

    // Load configuration
    let config = match config_file::load_config_file(config_path) {
        Ok(config) => config,
        Err(config_file::ConfigError::Io(ref io_err))
            if io_err.kind() == std::io::ErrorKind::NotFound =>
        {
            // If no config file was explicitly specified and mathcore.toml doesn't exist, use default
            if args.config_file.is_none() {
                config_file::Config::default()
            } else {
                // If a config file was explicitly specified but doesn't exist, that's an error
                eprintln!("Config file '{}' not found", config_path.display());
                std::process::exit(3);
            }
        }
        Err(err) => {
            // Any other error (parsing, permission, etc.) is always an error
            eprintln!(
                "Failed to load config file '{}': {}",
                config_path.display(),
                err
            );
            std::process::exit(4);
        }
    };

    let converter = LatexToMathML::new(config.math_core).unwrap_or_else(|err| {
        render_ariadne_report(&err.0, &format!("macro {}", err.1), &err.2);
        std::process::exit(2);
    });

    if let Some(fpath) = &args.file {
        let inline_delim: (&str, &str) = if let Some(open) = &args.inline_open {
            (open, args.inline_close.as_ref().unwrap())
        } else {
            (&args.inline_del, &args.inline_del)
        };
        let block_delim: (&str, &str) = if let Some(open) = &args.block_open {
            (open, args.block_close.as_ref().unwrap())
        } else {
            (&args.block_del, &args.block_del)
        };
        let replacer = Replacer::new(inline_delim, block_delim, args.ignore_escaped_delim);
        if fpath == &PathBuf::from("-") {
            let input = read_stdin();
            match replace(&replacer, &input, &converter, args.continue_on_error) {
                Ok(converted) => {
                    print_warnings(&converted.warnings, None);
                    println!("{}", converted.html);
                }
                Err(e) => exit_conversion_error(e, None),
            }
        } else if args.recursive {
            convert_html_recursive(&args, fpath, &replacer, &converter);
        } else {
            convert_html(&args, fpath, args.write, &replacer, &converter);
        }
    } else if let Some(formula) = &args.formula {
        convert_and_exit(&args, formula, &converter);
    } else {
        convert_and_exit(&args, &read_stdin(), &converter);
    }
}

fn read_stdin() -> String {
    let mut buffer = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        exit_io_error(&e);
    }
    buffer
}

fn convert_and_exit(args: &Args, latex: &str, converter: &LatexToMathML) {
    let display = if args.block {
        MathDisplay::Block
    } else {
        MathDisplay::Inline
    };
    match converter.convert_with_local_state(latex, display) {
        Ok(mathml) => println!("{}", mathml.mathml),
        Err(e) => {
            render_ariadne_report(&e, "<input>", latex);
            std::process::exit(2);
        }
    }
}

/// Find all LaTeX equations in a document and replace them with MathML.
///
/// The delimiters are configured by the `replacer` argument; a common configuration is `("$", "$")`
/// for inline equations and `("$$", "$$")` for block equations.
///
/// The document is first scanned in full and only then converted, because
/// [`LatexToMathML::convert_all`] needs all snippets of the document at once in order to resolve
/// references to equations that are only defined further down the document.
///
/// Note that delimiter characters that do not enclose a LaTeX equation (e.g. `This apple is $3.`)
/// must not appear in the input. Please use `&dollar;` instead of `$` outside LaTeX equations.
fn replace<'source>(
    replacer: &Replacer,
    input: &'source str,
    converter: &LatexToMathML,
    continue_on_error: bool,
) -> Result<Converted, ConversionError<'source>> {
    let scan = replacer.scan(input)?;
    let converted = converter.convert_all(&scan.snippets);

    let mut result = String::with_capacity(input.len());
    let mut warnings = Vec::new();
    // One tracker for the whole document: the sites are visited in document order, so it never
    // has to look back at a part of the input it has already passed.
    let mut tracker = LineTracker::new(input);
    for (((latex, display), site), converted) in
        scan.snippets.into_iter().zip(&scan.sites).zip(converted)
    {
        result += site.preceding_text;
        match converted {
            Ok(converted) => {
                result += &converted.mathml;
                warnings.extend(SnippetWarning::for_site(
                    &mut tracker,
                    site,
                    converted.warnings,
                ));
            }
            Err(err) if continue_on_error => {
                result += &err.to_html(&latex, display, None);
            }
            Err(err) => return Err(ConversionError::latex_error(input, site, latex, *err)),
        }
    }
    result += scan.trailing_text;
    Ok(Converted {
        html: result,
        warnings,
    })
}

/// A converted document, together with everything worth telling the user about the conversion.
struct Converted {
    html: String,
    /// The warnings of all snippets of the document, in document order.
    warnings: Vec<SnippetWarning>,
}

/// Convert all LaTeX equations in all HTML files under a given directory.
///
/// The argument can be a file name or a directory name; in the latter case, all HTML files in the
/// directory are converted, recursively. The extension of HTML files must be `.html`; `.htm` files
/// are ignored.
///
/// Every file is converted on its own, so equation numbering starts from 1 in each file and
/// references are only resolved within the file they appear in.
fn convert_html_recursive(
    args: &Args,
    path: &Path,
    replacer: &Replacer,
    converter: &LatexToMathML,
) {
    if path.is_dir() {
        let dir = fs::read_dir(path).unwrap_or_else(|e| exit_io_error(&e));
        for entry in dir.filter_map(Result::ok) {
            convert_html_recursive(args, entry.path().as_ref(), replacer, converter)
        }
    } else if path.is_file()
        && let Some(ext) = path.extension()
        && ext == "html"
    {
        // In recursive mode we always write back to the files, since dumping
        // multiple files to stdout would not be useful.
        convert_html(args, path, true, replacer, converter);
    }
}

fn convert_html(
    args: &Args,
    fp: &Path,
    write: bool,
    replacer: &Replacer,
    converter: &LatexToMathML,
) {
    let original = fs::read_to_string(fp).unwrap_or_else(|e| exit_io_error(&e));
    let converted = replace(replacer, &original, converter, args.continue_on_error)
        .unwrap_or_else(|e| exit_conversion_error(e, Some(fp)));
    print_warnings(&converted.warnings, Some(fp));
    if args.dry_run {
        return;
    }
    if write {
        if original != converted.html {
            let mut fp = fs::File::create(fp).unwrap_or_else(|e| exit_io_error(&e));
            fp.write_all(converted.html.as_bytes())
                .unwrap_or_else(|e| exit_io_error(&e));
        }
    } else {
        print!("{}", converted.html);
    }
}

/// Whether error reports should be colorized.
///
/// The rules are the same ones `clap` applies to its own colored output, so that the whole binary
/// behaves consistently. See [`decide_color`].
pub(crate) fn use_color() -> bool {
    /// The convention both color variables use: set to anything other than the empty string.
    fn is_set(var: &str) -> bool {
        std::env::var_os(var).is_some_and(|value| !value.is_empty())
    }

    /// Whether `TERM` describes a terminal that cannot display colors.
    ///
    /// On Windows `TERM` is usually not set even though the console does support colors, so the
    /// variable is only consulted on unix.
    fn terminal_is_dumb() -> bool {
        cfg!(unix)
            && match std::env::var_os("TERM") {
                Some(term) => term == "dumb",
                None => true,
            }
    }

    static USE_COLOR: LazyLock<bool> = LazyLock::new(|| {
        decide_color(
            is_set("NO_COLOR"),
            is_set("CLICOLOR_FORCE"),
            terminal_is_dumb(),
            // Both kinds of report are written to stderr.
            std::io::stderr().is_terminal(),
        )
    });
    *USE_COLOR
}

/// Decide whether to colorize, given the state of the environment.
///
/// 1. `NO_COLOR` turns colors off (<https://no-color.org/>).
/// 2. Otherwise `CLICOLOR_FORCE` turns them on. This is how colors can be kept when piping into a
///    pager or capturing a CI log.
/// 3. Otherwise colors are used only when stderr is a terminal that can display them.
fn decide_color(
    no_color: bool,
    clicolor_force: bool,
    terminal_is_dumb: bool,
    is_terminal: bool,
) -> bool {
    !no_color && (clicolor_force || (is_terminal && !terminal_is_dumb))
}

fn render_ariadne_report(error: &LatexError, source_name: &str, input: &str) {
    let report = error.to_report(source_name, use_color());
    report
        .eprint((source_name, ariadne::Source::from(input)))
        .expect("failed to write report");
}

/// Report the warnings of a converted document on stderr, one per line.
///
/// The whole document is converted before any of it is written out, so every warning is known by
/// the time the first byte of the result is printed. Emitting them here therefore puts all of them
/// ahead of the document itself, which keeps them readable when stdout and stderr both go to the
/// same terminal. Writing the document in pieces would lose that property.
fn print_warnings(warnings: &[SnippetWarning], fp: Option<&Path>) {
    for warning in warnings {
        eprint!("Warning");
        if let Some(fp) = fp {
            eprint!(" in '{}'", fp.display());
        }
        eprintln!(": {warning}");
    }
}

fn exit_conversion_error<E: std::error::Error>(e: E, fp: Option<&Path>) -> ! {
    eprint!("Conversion error");
    if let Some(fp) = fp {
        eprint!(" in '{}'", fp.display());
    }
    eprintln!(": {e}");
    std::process::exit(2);
}

fn exit_io_error(e: &std::io::Error) -> ! {
    eprintln!("IO Error: {e}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {

    /// Check that the `clap` argument definitions are internally consistent, e.g. that every
    /// `conflicts_with` and `requires` refers to an argument that actually exists.
    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;

        crate::Args::command().debug_assert();
    }

    /// The full truth table of [`crate::decide_color`]. The wiring to the actual environment is
    /// covered end to end by `tests/cli.rs`; the dumb-terminal case needs a real terminal and so
    /// can only be checked here.
    #[test]
    fn color_decision() {
        use crate::decide_color;

        // NO_COLOR beats everything else.
        for &clicolor_force in &[false, true] {
            for &dumb in &[false, true] {
                for &is_terminal in &[false, true] {
                    assert!(!decide_color(true, clicolor_force, dumb, is_terminal));
                }
            }
        }
        // CLICOLOR_FORCE beats the terminal detection, dumb terminal included.
        assert!(decide_color(false, true, false, false));
        assert!(decide_color(false, true, true, false));
        // Otherwise: only a non-dumb terminal gets colors.
        assert!(decide_color(false, false, false, true));
        assert!(!decide_color(false, false, true, true));
        assert!(!decide_color(false, false, false, false));
    }

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
        let converter =
            math_core::LatexToMathML::new(math_core::MathCoreConfig::default()).unwrap();
        let replacer = crate::Replacer::new(("$", "$"), ("$$", "$$"), false);
        let mathml = crate::replace(&replacer, text, &converter, false).unwrap();
        println!("{}", mathml.html);
    }

    /// A reference to an equation that is only defined further down the document has to resolve.
    /// This is only possible because the whole document is converted in one go.
    #[test]
    fn forward_reference() {
        let text = r"<p>See $\eqref{eq:a}$.</p>
<p>$$\begin{align} x = 1 \label{eq:a}\end{align}$$</p>";
        let converter =
            math_core::LatexToMathML::new(math_core::MathCoreConfig::default()).unwrap();
        let replacer = crate::Replacer::new(("$", "$"), ("$$", "$$"), false);
        let converted = crate::replace(&replacer, text, &converter, false).unwrap();
        // The `\eqref` has to render as a reference to equation (1), not as an unresolved one.
        let reference = converted.html.split_once("</p>").unwrap().0;
        assert!(
            reference.contains("(1)"),
            "unresolved reference: {reference}"
        );
    }

    #[test]
    fn continue_on_error() {
        let text = r"good $x$, bad $\frac$, good $y$";
        let converter =
            math_core::LatexToMathML::new(math_core::MathCoreConfig::default()).unwrap();
        let replacer = crate::Replacer::new(("$", "$"), ("$$", "$$"), false);
        // Without `continue_on_error`, the bad snippet aborts the whole conversion.
        assert!(crate::replace(&replacer, text, &converter, false).is_err());
        // With it, the error is rendered inline and the other snippets still convert.
        let mathml = crate::replace(&replacer, text, &converter, true)
            .unwrap()
            .html;
        assert!(mathml.starts_with("good <math"));
        assert!(mathml.ends_with("</math>"));
    }
}
