//! End-to-end tests that run the `mathcore` binary itself.
//!
//! The unit tests in `main.rs` call `replace` directly and therefore cover none of the argument
//! parsing, config discovery, file handling or exit codes. These tests drive the actual binary
//! that cargo builds for this test run, via `CARGO_BIN_EXE_mathcore`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// `pass_stdin` resolves through the `SpawnExt` trait that `assert_cmd_snapshot!` brings into
// scope itself, so the trait does not need to be imported here.
use insta_cmd::assert_cmd_snapshot;
use tempfile::TempDir;

/// A document whose *first* formula refers to an equation that only a *later* formula defines.
const FORWARD_REF_DOC: &str = r"<p>See $\eqref{eq:a}$.</p>
<p>$$\begin{align} x = 1 \label{eq:a}\end{align}$$</p>
";

/// A document with a single numbered equation and a reference to it.
const NUMBERED_DOC: &str = r"<p>$$\begin{align} a = 1 \label{eq:x}\end{align}$$ and $\eqref{eq:x}$</p>
";

/// Run the `mathcore` binary that cargo built for this test run.
///
/// The working directory is always pinned by the caller: the CLI looks for `mathcore.toml`
/// relative to the current directory, and the repository root has one, so an inherited working
/// directory would silently change the output.
fn mathcore(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mathcore"));
    cmd.current_dir(dir);
    cmd
}

/// A directory that is guaranteed not to contain a `mathcore.toml`, so the defaults apply.
fn empty_dir() -> TempDir {
    TempDir::new().expect("failed to create temporary directory")
}

/// Snapshot settings that drop the ANSI color codes from the `ariadne` error reports, which the
/// CLI emits unconditionally.
fn settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.set_strip_ansi_escape_codes(true);
    settings
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create directory");
    }
    fs::write(&path, content).expect("failed to write file");
    path
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("failed to read file")
}

// ---------------------------------------------------------------------------------------------
// Single-formula mode
// ---------------------------------------------------------------------------------------------

#[test]
fn formula_block() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(mathcore(dir.path()).args(["--formula", "x^2", "--block"]));
    });
}

#[test]
fn formula_from_stdin() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(mathcore(dir.path()).pass_stdin("a + b"));
    });
}

#[test]
fn formula_error_report() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(mathcore(dir.path()).args(["--formula", r"\frac"]));
    });
}

// ---------------------------------------------------------------------------------------------
// HTML mode via stdin
// ---------------------------------------------------------------------------------------------

#[test]
fn html_from_stdin() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .arg("-")
                .pass_stdin("inline $a+b$ and block $$c$$\n")
        );
    });
}

/// The whole document is converted in one batch, so a reference to an equation further down the
/// document resolves to its number instead of `(??)`.
#[test]
fn forward_reference_resolves() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(mathcore(dir.path()).arg("-").pass_stdin(FORWARD_REF_DOC));
    });
}

#[test]
fn html_entities_are_decoded() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .arg("-")
                .pass_stdin("$a &lt; b$ and $x &gt; y$\n")
        );
    });
}

#[test]
fn custom_delimiters() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .args(["--inline-open", r"\(", "--inline-close", r"\)", "-"])
                .pass_stdin(r"let \(a=1\) and $not math$")
        );
    });
}

// ---------------------------------------------------------------------------------------------
// Error reporting
// ---------------------------------------------------------------------------------------------

#[test]
fn latex_error_aborts_with_report() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .arg("-")
                .pass_stdin("good $x$ then bad $\\frac$\n")
        );
    });
}

#[test]
fn continue_on_error_inlines_the_error() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .args(["-", "--continue-on-error"])
                .pass_stdin("good $x$ then bad $\\frac$\n")
        );
    });
}

#[test]
fn unclosed_delimiter() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .arg("-")
                .pass_stdin("unclosed $delim\n")
        );
    });
}

#[test]
fn mismatched_delimiters() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(
            mathcore(dir.path())
                .arg("-")
                .pass_stdin("mismatch $$ and $ signs\n")
        );
    });
}

/// Colorization of the error reports follows the same rules as `clap`'s own colored output:
/// `NO_COLOR` wins over `CLICOLOR_FORCE`, which wins over the terminal detection, and both
/// variables count as set when they hold any non-empty value.
///
/// Both places that render a report are checked: the single-formula path and the HTML path, which
/// reports the error for one snippet of a document.
#[test]
fn color_follows_the_environment() {
    /// `stderr` of a run that fails with a colorizable `ariadne` report.
    ///
    /// Note that the test harness captures stderr through a pipe, so the terminal detection says
    /// "no terminal" unless `CLICOLOR_FORCE` overrides it.
    fn stderr_of(html: bool, env: &[(&str, &str)]) -> Vec<u8> {
        let dir = empty_dir();
        let mut cmd = mathcore(dir.path());
        if html {
            write(dir.path(), "doc.html", "good $x$ then bad $\\frac$\n");
            cmd.arg("doc.html");
        } else {
            cmd.args(["--formula", r"\frac"]);
        }
        // Either variable may well be set in the environment running the test suite.
        cmd.env_remove("NO_COLOR").env_remove("CLICOLOR_FORCE");
        for (name, value) in env {
            cmd.env(name, value);
        }

        let out = cmd.output().expect("failed to run mathcore");
        assert_eq!(out.status.code(), Some(2), "expected a conversion error");
        out.stderr
    }

    /// Whether the output contains ANSI escape codes.
    fn colorized(stderr: &[u8]) -> bool {
        stderr.contains(&0x1b)
    }

    /// One row of the truth table.
    struct Case {
        env: &'static [(&'static str, &'static str)],
        /// Whether the report is expected to come out colorized.
        colorized: bool,
        reason: &'static str,
    }

    let cases = [
        Case {
            env: &[],
            colorized: false,
            reason: "stderr is not a terminal, so no colors",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "1")],
            colorized: true,
            reason: "CLICOLOR_FORCE forces colors",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "0")],
            colorized: true,
            reason: "CLICOLOR_FORCE counts as set for any non-empty value, even \"0\"",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "")],
            colorized: false,
            reason: "an empty CLICOLOR_FORCE is not set",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")],
            colorized: false,
            reason: "NO_COLOR wins over CLICOLOR_FORCE",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "0")],
            colorized: false,
            reason: "NO_COLOR counts as set for any non-empty value, even \"0\"",
        },
        Case {
            env: &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "")],
            colorized: true,
            reason: "an empty NO_COLOR is not set",
        },
    ];

    for html in [false, true] {
        for case in &cases {
            assert_eq!(
                colorized(&stderr_of(html, case.env)),
                case.colorized,
                "html={html}, env={:?}: {}",
                case.env,
                case.reason
            );
        }
    }
}

#[test]
fn missing_config_file() {
    let dir = empty_dir();
    settings().bind(|| {
        assert_cmd_snapshot!(mathcore(dir.path()).args([
            "--config-file",
            "nope.toml",
            "--formula",
            "x"
        ]));
    });
}

// ---------------------------------------------------------------------------------------------
// File handling
// ---------------------------------------------------------------------------------------------

#[test]
fn without_write_the_file_is_left_alone() {
    let dir = empty_dir();
    let file = write(dir.path(), "doc.html", NUMBERED_DOC);
    let out = mathcore(dir.path())
        .arg("doc.html")
        .output()
        .expect("failed to run mathcore");

    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("<math"));
    assert_eq!(read(&file), NUMBERED_DOC, "the input file was modified");
}

#[test]
fn write_converts_in_place() {
    let dir = empty_dir();
    let file = write(dir.path(), "doc.html", NUMBERED_DOC);
    let out = mathcore(dir.path())
        .args(["--write", "doc.html"])
        .output()
        .expect("failed to run mathcore");

    assert!(out.status.success());
    assert!(out.stdout.is_empty(), "--write should not print the result");
    assert!(read(&file).contains("<math"));
}

/// Converting an already-converted document must not keep changing it.
#[test]
fn write_is_idempotent() {
    let dir = empty_dir();
    let file = write(dir.path(), "doc.html", NUMBERED_DOC);
    for _ in 0..2 {
        assert!(
            mathcore(dir.path())
                .args(["--write", "doc.html"])
                .status()
                .expect("failed to run mathcore")
                .success()
        );
    }
    let once = read(&file);

    let fresh = empty_dir();
    let fresh_file = write(fresh.path(), "doc.html", NUMBERED_DOC);
    assert!(
        mathcore(fresh.path())
            .args(["--write", "doc.html"])
            .status()
            .expect("failed to run mathcore")
            .success()
    );
    assert_eq!(once, read(&fresh_file), "second run changed the document");
}

#[test]
fn dry_run_writes_nothing() {
    let dir = empty_dir();
    let file = write(dir.path(), "doc.html", NUMBERED_DOC);
    let out = mathcore(dir.path())
        .args(["--dry-run", "--write", "doc.html"])
        .output()
        .expect("failed to run mathcore");

    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    assert_eq!(read(&file), NUMBERED_DOC, "--dry-run modified the file");
}

#[test]
fn a_failing_file_is_left_untouched() {
    let dir = empty_dir();
    let file = write(dir.path(), "doc.html", "before $x$ and $\\frac$ after\n");
    let out = mathcore(dir.path())
        .args(["--write", "doc.html"])
        .output()
        .expect("failed to run mathcore");

    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("Conversion error in 'doc.html'"));
    assert_eq!(
        read(&file),
        "before $x$ and $\\frac$ after\n",
        "the file was modified despite the error"
    );
}

// ---------------------------------------------------------------------------------------------
// Recursive mode
// ---------------------------------------------------------------------------------------------

/// Each file is converted as its own document, so the equation counter restarts in every file
/// instead of continuing across the whole run.
#[test]
fn recursive_numbers_each_file_from_one() {
    let dir = empty_dir();
    let first = write(dir.path(), "one.html", NUMBERED_DOC);
    let second = write(dir.path(), "sub/two.html", NUMBERED_DOC);

    assert!(
        mathcore(dir.path())
            .args(["--recursive", "."])
            .status()
            .expect("failed to run mathcore")
            .success()
    );

    for file in [&first, &second] {
        let converted = read(file);
        assert!(converted.contains("(1)"), "not converted: {converted}");
        assert!(
            !converted.contains("(2)"),
            "numbering continued across files: {converted}"
        );
    }
}

#[test]
fn recursive_ignores_non_html_files() {
    let dir = empty_dir();
    let htm = write(dir.path(), "old.htm", NUMBERED_DOC);
    let txt = write(dir.path(), "notes.txt", NUMBERED_DOC);

    assert!(
        mathcore(dir.path())
            .args(["--recursive", "."])
            .status()
            .expect("failed to run mathcore")
            .success()
    );

    assert_eq!(read(&htm), NUMBERED_DOC, ".htm files must be ignored");
    assert_eq!(read(&txt), NUMBERED_DOC, ".txt files must be ignored");
}
