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
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_zero_arg", mathml, latex);
}
#[test]
fn test_one_arg() {
    let macros = vec![
        ("half".to_string(), r"\frac{1}{2}\mspace{3mu}".to_string()),
        ("mycmd".to_string(), r"\sqrt{#1}".to_string()),
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
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_spacing", mathml, latex);
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
        .convert_with_local_counter(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("custom_cmd_literal_args", mathml, latex);
}
