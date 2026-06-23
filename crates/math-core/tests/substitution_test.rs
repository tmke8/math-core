use insta::assert_snapshot;
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint, UnicodeSubstitution};

/// The commands that have two possible realizations depending on `unicode_substitution`:
/// a single combined Unicode character (`Conventional`) or their constituent parts (`Never`).
const PROBLEMS: &[(&str, &str)] = &[
    ("coloneq", r"a\coloneq b"),
    ("coloneqq", r"a\coloneqq b"),
    ("capital_coloneq", r"a\Coloneq b"),
    ("capital_coloneqq", r"a\Coloneqq b"),
    ("dashcolon", r"a\dashcolon b"),
    ("dblcolon", r"a\dblcolon b"),
    ("eqcolon", r"a\eqcolon b"),
    ("eqqcolon", r"a\eqqcolon b"),
    ("cdots_before_open", r"4 + \cdots ()"),
    ("cdots_before_close", r"{\sum \cdots}"),
];

fn convert_all(unicode_substitution: UnicodeSubstitution, suffix: &str) {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        unicode_substitution,
        // A macro that expands to one of the substituted commands, to confirm the
        // `unicode_substitution` flag is applied while tokenizing user-defined macros.
        macros: vec![(String::from("mycoloneqq"), String::from(r"\coloneqq"))],
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();
    for (name, problem) in PROBLEMS.iter().chain([&("via_macro", r"a\mycoloneqq b")]) {
        let mathml = converter
            .convert_with_local_state(problem, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{}` with error '{}'", problem, e));
        assert_snapshot!(format!("{name}--{suffix}"), &mathml, problem);
    }
}

#[test]
fn conventional_substitution() {
    convert_all(UnicodeSubstitution::Conventional, "conventional");
}

#[test]
fn never_substitution() {
    convert_all(UnicodeSubstitution::Never, "never");
}
