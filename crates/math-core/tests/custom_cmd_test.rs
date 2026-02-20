use insta::assert_snapshot;
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

#[test]
fn test_zero_arg() {
    let macros = vec![
        ("half".to_owned(), r"\frac{1}{2}".to_owned()),
        ("mycmd".to_owned(), r"\sqrt{3}".to_owned()),
        ("withText".to_owned(), r"\text{a b}\sum".to_owned()),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x = \half, \withText 3";
    let mathml = converter
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_zero_arg", mathml, latex);
}

#[test]
fn test_one_arg() {
    let macros = vec![
        ("half".to_owned(), r"\frac{1}{2}\mspace{3mu}".to_owned()),
        ("mycmd".to_owned(), r"\sqrt{#1}".to_owned()),
    ];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x = \mycmd{3} + \half";
    let mathml = converter
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_one_arg", mathml, latex);
}

#[test]
fn test_error() {
    let macros = vec![
        ("x".to_owned(), "x".to_owned()),
        ("mycmd".to_owned(), r"\sqrt{#}".to_owned()),
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
    let macros = vec![("eq".to_owned(), r"=".to_owned())];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x + \eq 3";
    let mathml = converter
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_spacing", mathml, latex);
}

#[test]
fn test_literal_args() {
    let macros = vec![("hs".to_owned(), r"\hspace{#1}".to_owned())];

    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };

    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"x \hs{3em} y";
    let mathml = converter
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_literal_args", mathml, latex);
}
