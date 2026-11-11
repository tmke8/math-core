use insta::assert_snapshot;
use math_core::{
    Indentation, LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint, TaggingStrategy,
};

#[test]
fn test_indentation_spaces() {
    // Two spaces per indentation level instead of the default four.
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        indentation: Indentation::Spaces(2),
        ..Default::default()
    };
    let converter = LatexToMathML::new(config, vec![]).unwrap();

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
        indentation: Indentation::Tab,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config, vec![]).unwrap();

    let mathml = converter
        .convert_with_local_state(r"\frac{1}{2}", MathDisplay::Block)
        .unwrap()
        .mathml;

    assert!(mathml.contains("\n\t<mfrac>"), "output was:\n{mathml}");
    // No space-based indentation should be emitted when using tabs.
    assert!(!mathml.contains("\n  "), "output was:\n{mathml}");
}

fn converter_with_tags(tags: TaggingStrategy) -> LatexToMathML {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        tags,
        ..Default::default()
    };
    LatexToMathML::new(config, vec![]).unwrap()
}

/// With `TaggingStrategy::None`, the only equation numbers are the ones set with `\tag`.
#[test]
fn test_tags_none() {
    let problems = [
        ("tags_none_equation", r"\begin{equation} x \end{equation}"),
        (
            "tags_none_align",
            r"\begin{align} a \\ b \tag{x} \\ c \end{align}",
        ),
        // Without a number, there is nothing which the label could refer to.
        (
            "tags_none_label",
            r"\begin{gather} a \label{eq:a} \\ b \tag*{B} \label{eq:b} \end{gather}",
        ),
        (
            "tags_none_multline",
            r"\begin{multline} a \\ b \end{multline}",
        ),
        // In `multline`, the tag is still shown in the last row.
        (
            "tags_none_multline_tag",
            r"\begin{multline} a \tag{x} \\ b \end{multline}",
        ),
        ("tags_none_plain", r"x = 1"),
    ];

    let converter = converter_with_tags(TaggingStrategy::None);
    for (name, problem) in problems {
        let mathml = converter
            .convert_with_local_state(problem, MathDisplay::Block)
            .unwrap_or_else(|e| panic!("failed to convert `{problem}` with error '{e}'"))
            .mathml;
        assert_snapshot!(name, &mathml, problem);
    }
}

/// With `TaggingStrategy::All`, a block equation without a numbered environment is numbered as
/// a whole.
#[test]
fn test_tags_all() {
    let problems = [
        ("tags_all_plain", r"\sin x = 1"),
        ("tags_all_tag", r"x = 1 \tag{A}"),
        ("tags_all_label", r"x = 1 \label{eq:x} \tag*{A}"),
        ("tags_all_notag", r"x = 1 \notag"),
        // Environments which can't be numbered are part of the equation...
        (
            "tags_all_aligned",
            r"\begin{aligned} a &= b \\ c &= d \end{aligned}",
        ),
        // ...while the display environments are the whole equation and do the numbering
        // themselves, even if they don't number anything.
        ("tags_all_align", r"\begin{align} a \\ b \end{align}"),
        (
            "tags_all_equation_star",
            r"\begin{equation*} x \end{equation*}",
        ),
    ];

    let converter = converter_with_tags(TaggingStrategy::All);
    for (name, problem) in problems {
        let mathml = converter
            .convert_with_local_state(problem, MathDisplay::Block)
            .unwrap_or_else(|e| panic!("failed to convert `{problem}` with error '{e}'"))
            .mathml;
        assert_snapshot!(name, &mathml, problem);
    }

    // The equation is numbered exactly as if it were the content of `equation`.
    let convert = |latex| {
        converter
            .convert_with_local_state(latex, MathDisplay::Block)
            .unwrap()
            .mathml
    };
    assert_eq!(
        convert(r"a + \sum_i x_i = 1 \label{eq:a}"),
        convert(r"\begin{equation} a + \sum_i x_i = 1 \label{eq:a} \end{equation}"),
    );

    // A tag or a label belongs to the equation as a whole, so a display environment, which
    // has to be the whole equation, cannot follow it.
    for problem in [
        r"\tag{A} \begin{align} x \end{align}",
        r"\label{eq:a} \begin{align} x \end{align}",
    ] {
        let Err(error) = converter.convert_with_local_state(problem, MathDisplay::Block) else {
            panic!("problem `{problem}` did not return an error");
        };
        assert_eq!(
            error.error_message(),
            "Environment \"align\" may only appear at the top level of a block equation, \
             with nothing before it.",
        );
    }

    // Inline equations are never numbered, so `\tag` stays an error there.
    let inline = converter
        .convert_with_local_state("x = 1", MathDisplay::Inline)
        .unwrap()
        .mathml;
    assert!(!inline.contains("mtable"), "output was:\n{inline}");
    assert!(
        converter
            .convert_with_local_state(r"x \tag{1}", MathDisplay::Inline)
            .is_err()
    );
}

/// The block equations numbered by `TaggingStrategy::All` share the counter with the numbered
/// environments, and can be referenced like them.
#[test]
fn test_tags_all_counter() {
    let snippets = [
        (r"\eqref{eq:d}", MathDisplay::Inline),
        (r"a", MathDisplay::Block),
        (r"\begin{align} b \\ c \end{align}", MathDisplay::Block),
        (r"d \label{eq:d}", MathDisplay::Block),
    ];
    let config = MathCoreConfig {
        tags: TaggingStrategy::All,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config, vec![]).unwrap();
    let results: Vec<String> = converter
        .convert_all(&snippets)
        .into_iter()
        .map(|result| result.unwrap().mathml)
        .collect();

    assert!(
        results[0].contains(r##"<a href="#eq:d">(4)</a>"##),
        "output was:\n{}",
        results[0]
    );
    assert!(results[1].contains("<mtext>(1)</mtext>"));
    assert!(results[2].contains("<mtext>(2)</mtext>"));
    assert!(results[2].contains("<mtext>(3)</mtext>"));
    assert!(!results[2].contains("<mtext>(4)</mtext>"));
    assert!(results[3].contains(r#"id="eq:d"><mtext>(4)</mtext>"#));
}

/// The default is to number equations as AMSmath does.
#[test]
fn test_tags_default() {
    let converter = LatexToMathML::new(MathCoreConfig::default(), vec![]).unwrap();
    let convert = |latex| {
        converter
            .convert_with_local_state(latex, MathDisplay::Block)
            .unwrap()
            .mathml
    };
    assert!(convert(r"\begin{equation} x \end{equation}").contains("<mtext>(1)</mtext>"));
    assert!(!convert(r"x = 1").contains("<mtext>(1)</mtext>"));
}
