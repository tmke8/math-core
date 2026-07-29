use insta::assert_snapshot;
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

#[test]
fn test_zero_arg() {
    let macros = vec![
        ("half".to_string(), r"\frac{1}{2}".to_string()),
        ("mycmd".to_string(), r"\sqrt{3}".to_string()),
        ("withText".to_string(), r"\text{a b}\sum".to_string()),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x = \half, \withText 3";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_zero_arg", mathml.mathml, latex);
}

#[test]
fn test_one_arg() {
    let macros = vec![
        ("half".to_string(), r"\frac{1}{2}\mspace{3mu}".to_string()),
        (
            "mycmd".to_string(),
            r"\begin{array}{c|c}#1 & 0\end{array}".to_string(),
        ),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x = \mycmd{3} + \half";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_one_arg", mathml.mathml, latex);
}

#[test]
fn test_error() {
    let macros = vec![
        ("x".to_string(), "x".to_string()),
        ("mycmd".to_string(), r"\sqrt{#}".to_string()),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let error = LatexToMathML::new(config).unwrap_err();

    assert_eq!(error.1, 1);
    assert_eq!(error.2, r"\sqrt{#}");
}

#[test]
fn test_spacing() {
    let macros = vec![("eq".to_string(), r"=".to_string())];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x + \eq 3";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_spacing", mathml.mathml, latex);
}

#[test]
fn test_empty_args() {
    let macros = vec![
        (
            "odv".to_string(),
            r"\frac{\mathrm d #1}{\mathrm d #2}".to_string(),
        ),
        ("bb".to_string(), r"\mathbb{#1}".to_string()),
        ("two".to_string(), r"#1+#2".to_string()),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        ("empty_first_arg", r"\odv{}{x}"),
        ("empty_second_arg", r"\odv{y}{}"),
        ("empty_only_arg", r"\bb{}"),
        ("empty_arg_at_stream_end", r"\two{a}{}"),
        ("empty_arg_at_stream_start", r"\two{}{b}"),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap();

        assert_snapshot!(name, mathml.mathml, latex);
    }
}

#[test]
fn test_literal_args() {
    let macros = vec![("hs".to_string(), r"\hspace{#1}".to_string())];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x \hs{3em} y";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_literal_args", mathml.mathml, latex);
}

#[test]
fn test_newcommand() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        (
            "newcommand_one_arg",
            r"\newcommand{\bb}[1]{\mathbb{#1}}\bb{C}",
        ),
        (
            "newcommand_zero_args",
            r"\newcommand{\zz}{\mathbb{Z}}x \in \zz",
        ),
        (
            "newcommand_two_args",
            r"\newcommand{\pair}[2]{(#1, #2)}\pair{x}{y}",
        ),
        // Without braces around the name, as `\newcommand` also allows.
        (
            "newcommand_unbraced_name",
            r"\newcommand\half{\frac{1}{2}}\half",
        ),
        // A predefined command which is itself a token stream.
        (
            "newcommand_predefined_cmd_in_body",
            r"\newcommand{\imp}{\implies}a \imp b",
        ),
        // A command defined by an earlier `\newcommand`, with an argument, which is the
        // case where the arguments of the two commands could clobber each other.
        (
            "newcommand_nested_cmd_in_body",
            r"\newcommand{\br}[1]{[#1]}\newcommand{\dbr}[1]{\br{\br{#1}}}\dbr{z}",
        ),
        // The class of the body determines the spacing, as it does for config macros.
        ("newcommand_spacing", r"\newcommand{\eq}{=}x + \eq 3"),
        (
            "newcommand_empty_arg",
            r"\newcommand{\bb}[1]{\mathbb{#1}}\bb{}",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

#[test]
fn test_newcommand_uses_config_macro() {
    let macros = vec![("half".to_string(), r"\frac{1}{2}".to_string())];
    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"\newcommand{\halves}[1]{#1\half}\halves{3}";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("newcommand_uses_config_macro", mathml.mathml, latex);
}

#[test]
fn test_newcommand_across_snippets() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    // The definition in the first snippet is available in the second one.
    converter
        .convert_with_global_state(
            r"\newcommand{\bb}[1]{\mathbb{#1}}\bb{R}",
            MathDisplay::Inline,
        )
        .unwrap();
    let mathml = converter
        .convert_with_global_state(r"\bb{C}", MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("newcommand_across_snippets", mathml.mathml, r"\bb{C}");

    // ... but not after the global state has been reset.
    converter.reset_global_state();
    assert!(
        converter
            .convert_with_global_state(r"\bb{C}", MathDisplay::Inline)
            .is_err()
    );
}

#[test]
fn test_newcommand_in_convert_all() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let results = converter.convert_all(&[
        (r"\newcommand{\zz}{\mathbb{Z}}", MathDisplay::Inline),
        (r"n \in \zz", MathDisplay::Inline),
    ]);
    let mathml = &results[1].as_ref().unwrap().mathml;
    assert_snapshot!("newcommand_in_convert_all", mathml, r"n \in \zz");
}

#[test]
fn test_newcommand_does_not_leak_into_local_state() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_local_state(r"\newcommand{\zz}{\mathbb{Z}}\zz", MathDisplay::Inline)
        .unwrap();
    assert!(
        converter
            .convert_with_local_state(r"\zz", MathDisplay::Inline)
            .is_err()
    );
}
