use math_core::{Indentation, LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

#[test]
fn test_indentation_spaces() {
    // Two spaces per indentation level instead of the default four.
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        indentation: Indentation::Spaces(2),
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let mathml = converter
        .convert_with_local_state(r"\frac{1}{2}", MathDisplay::Block)
        .unwrap()
        .mathml;

    // The first level of nesting is indented with exactly two spaces instead of the
    // default four (deeper levels are multiples of the unit, so we check this specific line).
    assert!(mathml.contains("\n  <mfrac>"), "output was:\n{mathml}");
    assert!(!mathml.contains("\n    <mfrac>"), "output was:\n{mathml}");
}

#[test]
fn test_indentation_tab() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        indentation: Indentation::tab(),
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let mathml = converter
        .convert_with_local_state(r"\frac{1}{2}", MathDisplay::Block)
        .unwrap()
        .mathml;

    assert!(mathml.contains("\n\t<mfrac>"), "output was:\n{mathml}");
    // No space-based indentation should be emitted when using tabs.
    assert!(!mathml.contains("\n  "), "output was:\n{mathml}");
}
