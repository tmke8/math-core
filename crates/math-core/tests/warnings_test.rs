use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};

#[test]
fn test_undefined_reference_warning() {
    let config = MathCoreConfig::default();
    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"\eqref{doesnotexist}";
    let result = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert!(result.warnings.has_any());
    assert!(result.warnings.has_undefined_references());
    assert!(!result.warnings.has_unknown_commands());
}

#[test]
fn test_unknown_command_warning() {
    // Unknown commands are an error by default; `ignore_unknown_commands` makes them a warning
    // instead so the conversion succeeds.
    let config = MathCoreConfig {
        ignore_unknown_commands: true,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"\thiscommanddoesnotexist";
    let result = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert!(result.warnings.has_any());
    assert!(result.warnings.has_unknown_commands());
    assert!(!result.warnings.has_undefined_references());
}
