# math-core CLI

This is the command-line interface (CLI) for the [math-core library](https://crates.io/crates/math-core), a Rust-based converter that transforms LaTeX math expressions into the MathML Core format.

Once installed, you can use the `mathcore` command in your terminal. Use the `--help` flag to see available options and usage instructions:

A config file, `mathcore.toml` can be used to define custom LaTeX macros.

Error reports are colorized when the terminal supports it, following the same conventions as the
`--help` output:

- `NO_COLOR` set to any non-empty value turns the colors off ([no-color.org](https://no-color.org/)).
- Otherwise `CLICOLOR_FORCE` set to any non-empty value turns them on, which is useful when piping
  into a pager or capturing a CI log.
- Otherwise the colors are used only when standard error is a terminal that can display them.

See the [main README](https://crates.io/crates/math-core) for more information about the project.
