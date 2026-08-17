use insta::assert_snapshot;
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, MaxExpansions, PrettyPrint};

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
        // A command which takes an argument, followed by a reference to the argument of the
        // command being expanded. The inner command must not consume the outer argument.
        (
            "newcommand_arg_after_nested_cmd",
            r"\newcommand{\zoo}[1]{\ket{a}#1}\zoo{b}",
        ),
        // The same, but with the argument reference inside the nested command as well, and
        // with the two arguments in the opposite order.
        (
            "newcommand_arg_in_and_after_nested_cmd",
            r"\newcommand{\zoo}[2]{\ket{#2}#1}\zoo{b}{c}",
        ),
        // A predefined command which takes an argument, in the same position.
        (
            "newcommand_arg_after_predefined_cmd",
            r"\newcommand{\zoo}[1]{\pod{a}#1}\zoo{b}",
        ),
        // `\text` rejects anything that isn't a text-mode token, so an argument reference
        // inside it only works because the argument is substituted before `\text` sees it.
        (
            "newcommand_arg_in_text",
            r"\newcommand{\txt}[1]{\text{a#1b}}\txt{xy}",
        ),
        // A body which is empty expands to `\relax`, and must not swallow what comes after
        // it. The `\relax` still shows up as an empty row here, because the first token of
        // an expansion goes through `parse_token` rather than through the sequence handling
        // which drops a `\relax`.
        ("newcommand_empty_body", r"\newcommand{\nop}{}a\nop b"),
        // The same command as an argument, where it has to produce an empty group.
        ("newcommand_empty_body_as_arg", r"\newcommand{\nop}{}x^\nop"),
        // An empty argument is equivalent to `{}`, so it still counts as an argument for the
        // construct it is substituted into.
        (
            "newcommand_empty_arg_as_group",
            r"\newcommand{\p}[1]{x^#1}\p{}",
        ),
        // The class of the body determines the spacing, as it does for config macros.
        ("newcommand_spacing", r"\newcommand{\eq}{=}x + \eq 3"),
        (
            "newcommand_empty_arg",
            r"\newcommand{\bb}[1]{\mathbb{#1}}\bb{}",
        ),
        // `\mathchoice` must not clobber the arguments of the command it appears in, neither
        // when it comes before the argument reference nor when it contains one itself.
        (
            "newcommand_mathchoice_in_body",
            r"\newcommand{\mc}[1]{\mathchoice{D}{T}{S}{SS}#1\mathchoice{#1}{[#1]}{#1}{#1}}\mc{z}",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

#[test]
fn test_providecommand() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        // For a name which isn't taken, `\providecommand` behaves like `\newcommand`.
        (
            "providecommand_zero_args",
            r"\providecommand{\zz}{\mathbb{Z}}x \in \zz",
        ),
        (
            "providecommand_one_arg",
            r"\providecommand{\bb}[1]{\mathbb{#1}}\bb{C}",
        ),
        (
            "providecommand_unbraced_name",
            r"\providecommand\half{\frac{1}{2}}\half",
        ),
        // If the name is taken, the existing definition wins and the new one is discarded.
        (
            "providecommand_existing_custom_cmd",
            r"\newcommand{\zz}{\mathbb{Z}}\providecommand{\zz}{\mathbb{Q}}\zz",
        ),
        (
            "providecommand_existing_builtin",
            r"\providecommand{\frac}[2]{#1/#2}\frac{1}{2}",
        ),
        // A discarded definition doesn't stop a later `\newcommand` of the same name.
        (
            "providecommand_then_newcommand",
            r"\providecommand{\alpha}{a}\newcommand{\zz}{\mathbb{Z}}\zz",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

#[test]
fn test_renewcommand() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        (
            "renewcommand_replaces_newcommand",
            r"\newcommand{\zz}{\mathbb{Z}}\renewcommand{\zz}{\mathbb{Q}}\zz",
        ),
        // The command is used directly after the redefinition, so the token for it has
        // already been read with the old definition in place.
        (
            "renewcommand_used_immediately",
            r"\newcommand{\zz}{\mathbb{Z}}\renewcommand{\zz}{\mathbb{Q}}\zz\zz",
        ),
        // The number of arguments may change, too.
        (
            "renewcommand_changes_num_args",
            r"\newcommand{\bb}{\mathbb{Z}}\renewcommand{\bb}[1]{\mathbb{#1}}\bb{C}",
        ),
        (
            "renewcommand_unbraced_name",
            r"\newcommand\half{\frac{1}{2}}\renewcommand\half{\frac{1}{3}}\half",
        ),
        // A builtin command can be redefined as well.
        (
            "renewcommand_replaces_builtin",
            r"\renewcommand{\epsilon}{\varepsilon}\epsilon",
        ),
        (
            "renewcommand_replaces_builtin_with_args",
            r"\renewcommand{\frac}[2]{#1/#2}\frac{1}{2}",
        ),
        // A body refers to the names it mentions, not to what they meant when it was
        // recorded, so redefining a command does change the commands which use it, as it
        // does in LaTeX.
        (
            "renewcommand_affects_earlier_body",
            r"\newcommand{\xx}{1}\newcommand{\yy}{\xx}\renewcommand{\xx}{2}\yy",
        ),
        // After a `\renewcommand`, `\providecommand` still finds the name taken.
        (
            "renewcommand_then_providecommand",
            r"\renewcommand{\epsilon}{a}\providecommand{\epsilon}{b}\epsilon",
        ),
        // Redefining a command in terms of a command which is *not* the one being redefined
        // is not recursion, even when the two look alike.
        (
            "renewcommand_body_uses_other_command",
            r"\newcommand{\zz}{\mathbb{Z}}\renewcommand{\zz}{(\mathbb{Z})}\zz",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

/// A command in the body of a `\newcommand` is recorded by name, so it means whatever that
/// name means when the command is expanded, the way it does in LaTeX. This is what config
/// macros have always done; these are the cases where a document definition used to differ.
#[test]
fn test_late_resolution() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        // A builtin which is redefined after the body was recorded.
        (
            "late_resolution_of_builtin",
            r"\newcommand{\foo}{\epsilon}\renewcommand{\epsilon}{e}\foo",
        ),
        // The same, with neither name in braces.
        (
            "late_resolution_unbraced_name",
            r"\newcommand\foo{\epsilon}\renewcommand\epsilon{e}\foo",
        ),
        // A body which isn't a group consists of a single token, which is recorded by name
        // just like the tokens of a group are.
        (
            "late_resolution_unbraced_body",
            r"\newcommand\foo\epsilon\renewcommand\epsilon{e}\foo",
        ),
        // The definition is assembled out of the argument of another command, so its body
        // reaches the queue by substitution rather than straight from the lexer.
        (
            "late_resolution_through_argument",
            r"\newcommand{\m}[1]{x#1}\newcommand{\a}{1}\newcommand{\z}{0}\m{\renewcommand{\z}{\a}}\renewcommand{\a}{2}\z",
        ),
        // `\begin` and `\end` get their meaning in the lexer, which consumes the environment
        // name along with them, so they have no name to be recorded under. `\\` does.
        (
            "late_resolution_env_in_body",
            r"\newcommand{\m}{\begin{matrix}a\\b\end{matrix}}\m",
        ),
        // Two commands with an empty body mean the same thing without being the same command.
        (
            "late_resolution_empty_bodies",
            r"\newcommand{\p}{}\newcommand{\q}{}\newcommand{\pq}{a\p b\q c}\renewcommand{\q}{!}\pq",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

/// The same, across snippets: the name a body refers to has to outlive the snippet which
/// recorded it, and the redefinition which it picks up comes from a later snippet.
#[test]
fn test_late_resolution_across_snippets() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        global_group: true,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(r"\newcommand{\foo}{\epsilon}", MathDisplay::Inline)
        .unwrap();
    converter
        .convert_with_global_state(r"\renewcommand{\epsilon}{e}", MathDisplay::Inline)
        .unwrap();
    let mathml = converter
        .convert_with_global_state(r"\foo", MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("late_resolution_across_snippets", mathml.mathml, r"\foo");
}

#[test]
fn test_forward_reference() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        // The body of a command may name a command which doesn't exist yet; the name is
        // resolved when the command is used.
        (
            "forward_reference",
            r"\newcommand{\foo}{x+\baz}\newcommand{\baz}{y}\foo",
        ),
        // The same, with the forward reference inside a nested group and with arguments.
        (
            "forward_reference_with_args",
            r"\newcommand{\foo}[1]{\baz{#1}}\newcommand{\baz}[1]{[#1]}\foo{z}",
        ),
        // A forward reference is resolved every time the command is expanded, so the two uses
        // here get different definitions.
        (
            "forward_reference_resolved_at_use",
            r"\newcommand{\foo}{\baz}\newcommand{\baz}{1}\foo\renewcommand{\baz}{2}\foo",
        ),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }

    // A command which is never defined is an error, but only once it is used.
    converter
        .convert_with_local_state(r"\newcommand{\foo}{x+\baz}", MathDisplay::Inline)
        .unwrap();
    assert!(
        converter
            .convert_with_local_state(r"\newcommand{\foo}{x+\baz}\foo", MathDisplay::Inline)
            .is_err()
    );
}

#[test]
fn test_forward_reference_across_snippets() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        global_group: true,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    // The name which the definition refers to is defined in a later snippet, so the name has
    // to survive the snippet which mentions it.
    converter
        .convert_with_global_state(r"\newcommand{\foo}{x+\baz}", MathDisplay::Inline)
        .unwrap();
    converter
        .convert_with_global_state(r"\newcommand{\baz}{y}", MathDisplay::Inline)
        .unwrap();
    let mathml = converter
        .convert_with_global_state(r"\foo", MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("forward_reference_across_snippets", mathml.mathml, r"\foo");

    // ... but not after the global state has been reset.
    converter.reset_global_state();
    assert!(
        converter
            .convert_with_global_state(r"\foo", MathDisplay::Inline)
            .is_err()
    );
}

#[test]
fn test_expansion_limit() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for latex in [
        // Two commands which expand to each other.
        r"\newcommand{\a}{\b}\newcommand{\b}{\a}\a",
        // A command which expands to itself.
        r"\newcommand{\a}{x\a}\a",
        // A `\renewcommand` whose body mentions the command being redefined. The body refers
        // to the new definition, not to the old one, so this recurses, as it does in LaTeX.
        r"\newcommand{\zz}{\mathbb{Z}}\renewcommand{\zz}{(\zz)}\zz",
    ] {
        let Err(err) = converter.convert_with_local_state(latex, MathDisplay::Inline) else {
            panic!("`{latex}` should not have converted");
        };
        assert!(
            err.to_string().contains("Too many expansions"),
            "unexpected error for `{latex}`: {err}"
        );
    }
}

#[test]
fn test_configurable_expansion_limit() {
    // This terminates, but it needs three expansions: `\a`, and then `\b` twice.
    let latex = r"\newcommand{\a}{\b\b}\newcommand{\b}{x}\a";

    let strict = LatexToMathML::new(MathCoreConfig {
        max_expansions: MaxExpansions(2),
        ..Default::default()
    })
    .unwrap();
    let Err(err) = strict.convert_with_local_state(latex, MathDisplay::Inline) else {
        panic!("`{latex}` should not have converted with a limit of 2 expansions");
    };
    assert!(
        err.to_string().contains("Too many expansions"),
        "unexpected error: {err}"
    );

    // With a higher limit, the same input converts fine.
    let lenient = LatexToMathML::new(MathCoreConfig {
        max_expansions: MaxExpansions(100),
        ..Default::default()
    })
    .unwrap();
    assert!(
        lenient
            .convert_with_local_state(latex, MathDisplay::Inline)
            .is_ok()
    );
}

#[test]
fn test_config_macros_referring_to_each_other() {
    let macros = vec![
        // The macro which is defined first refers to the one which is defined later.
        ("half".to_string(), r"\frac{1}{\two}".to_string()),
        ("two".to_string(), r"2".to_string()),
    ];
    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let latex = r"\half";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!(
        "config_macros_referring_to_each_other",
        mathml.mathml,
        latex
    );
}

#[test]
fn test_config_macro_with_undefined_command() {
    let macros = vec![
        ("half".to_string(), r"\frac{1}{2}".to_string()),
        ("bad".to_string(), r"\frac{1}{\nosuchcommand}".to_string()),
    ];
    let config = MathCoreConfig {
        macros,
        ..Default::default()
    };

    // A name which none of the macros defines is still reported right away.
    let error = LatexToMathML::new(config).unwrap_err();
    assert_eq!(error.1, 1);
    assert_eq!(error.2, r"\frac{1}{\nosuchcommand}");
}

#[test]
fn test_renewcommand_config_macro() {
    let macros = vec![("half".to_string(), r"\frac{1}{2}".to_string())];
    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    // A macro from the configuration can be redefined, ...
    let latex = r"\renewcommand{\half}{\frac{1}{3}}\half";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("renewcommand_replaces_config_macro", mathml.mathml, latex);

    // ... but only for the snippet which does it, because the configuration is immutable.
    let latex = r"\half";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("renewcommand_config_macro_after", mathml.mathml, latex);
}

#[test]
fn test_renewcommand_unreliable_rendering() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        allow_unreliable_rendering: true,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    // Overwrite a command that is only available when `allow_unreliable_rendering` is set.
    let latex = r"\renewcommand{\widecheck}{x}\widecheck";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!(
        "renewcommand_replaces_builtin_unreliable",
        mathml.mathml,
        latex
    );
}

#[test]
fn test_renewcommand_across_snippets() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        global_group: true,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(r"\newcommand{\zz}{\mathbb{Z}}\zz", MathDisplay::Inline)
        .unwrap();
    // A definition from an earlier snippet can be replaced, ...
    converter
        .convert_with_global_state(r"\renewcommand{\zz}{\mathbb{Q}}", MathDisplay::Inline)
        .unwrap();
    let mathml = converter
        .convert_with_global_state(r"\zz", MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("renewcommand_across_snippets", mathml.mathml, r"\zz");

    // ... but not after the global state has been reset.
    converter.reset_global_state();
    assert!(
        converter
            .convert_with_global_state(r"\renewcommand{\zz}{\mathbb{R}}", MathDisplay::Inline)
            .is_err()
    );
}

#[test]
fn test_providecommand_with_config_macro() {
    let macros = vec![("half".to_string(), r"\frac{1}{2}".to_string())];
    let config = MathCoreConfig {
        macros,
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    // A macro from the configuration also counts as already defined.
    let latex = r"\providecommand{\half}{\frac{1}{3}}\half";
    let mathml = converter
        .convert_with_local_state(latex, MathDisplay::Inline)
        .unwrap();

    assert_snapshot!("providecommand_existing_config_macro", mathml.mathml, latex);
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
        global_group: true,
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
        global_group: true,
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
fn test_newcommand_without_global_group() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    // Outside of the global group, which is the default, a definition is only good for the
    // snippet which contains it, ...
    let latex = r"\newcommand{\bb}[1]{\mathbb{#1}}\bb{R}";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("newcommand_without_global_group", mathml.mathml, latex);

    // ... so the next snippet doesn't know it, ...
    assert!(
        converter
            .convert_with_global_state(r"\bb{C}", MathDisplay::Inline)
            .is_err()
    );
    // ... and can define it again itself.
    converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
}

#[test]
fn test_renewcommand_without_global_group() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(r"\newcommand{\zz}{\mathbb{Z}}\zz", MathDisplay::Inline)
        .unwrap();
    // The definition from the earlier snippet is gone, so there is nothing to replace.
    assert!(
        converter
            .convert_with_global_state(r"\renewcommand{\zz}{\mathbb{Q}}", MathDisplay::Inline)
            .is_err()
    );
}

#[test]
fn test_newcommand_in_convert_all_without_global_group() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    let results = converter.convert_all(&[
        (r"\newcommand{\zz}{\mathbb{Z}}", MathDisplay::Inline),
        (r"n \in \zz", MathDisplay::Inline),
    ]);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
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

/// `\let` binds the meaning a name has right now, which is what a `\newcommand` body,
/// recorded by name, deliberately doesn't do.
#[test]
fn test_let() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    for (name, latex) in [
        // The reason `\let` exists: hold on to a definition one is about to replace. Without
        // it, the redefinition would refer to itself and run into the expansion limit.
        (
            "let_keep_old_meaning",
            r"\let\originalepsilon\epsilon\renewcommand{\epsilon}{1+\originalepsilon}\epsilon",
        ),
        // The `=` between the two names is optional, and so is the space around it.
        ("let_plain", r"\let\a\alpha\a"),
        ("let_equals", r"\let\a=\alpha\a"),
        ("let_equals_spaced", r"\let\a = \alpha \a"),
        // Only one `=` is skipped, so this binds `\eq` to `=` itself.
        ("let_to_equals", r"\let\eq==x\eq y"),
        // Early binding: `\a` keeps the meaning `\alpha` had at the `\let`, unlike
        // `\newcommand{\a}{\alpha}`, whose body would follow the redefinition.
        (
            "let_does_not_follow_redefinition",
            r"\let\a\alpha\renewcommand{\alpha}{\beta}\a",
        ),
        // A command which takes arguments keeps its arity.
        ("let_command_with_args", r"\let\f\frac\f{1}{2}"),
        // The source may be a custom command, in which case the reference to its body is
        // what gets copied.
        (
            "let_to_custom_cmd",
            r"\newcommand{\m}[1]{\sqrt{#1}}\let\n\m\n{2}",
        ),
        // `\let` and `\newcommand` write into the same place, so a name never means two
        // things at once, no matter which of them comes last.
        (
            "let_overwrites_newcommand",
            r"\newcommand{\a}{x}\let\a\alpha\a",
        ),
        (
            "renewcommand_overwrites_let",
            r"\let\a\alpha\renewcommand{\a}{x}\a",
        ),
        // A `\let` to a name which is itself `\let` copies the meaning, not the name, so
        // redefining the middle name afterwards leaves both of the others alone.
        ("let_chain", r"\let\a\alpha\let\b\a\renewcommand{\a}{x}\a\b"),
    ] {
        let mathml = converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{latex}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, latex);
    }
}

/// In the global group, a `\let` outlives the snippet which contains it, just like a
/// `\newcommand` does.
#[test]
fn test_let_across_snippets() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        global_group: true,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(r"\newcommand{\zz}{\mathbb{Z}}", MathDisplay::Inline)
        .unwrap();
    // The source is a command from an earlier snippet, so its body lives in the store which
    // outlives the snippets, and the reference to it stays good.
    converter
        .convert_with_global_state(r"\let\oldzz\zz", MathDisplay::Inline)
        .unwrap();
    converter
        .convert_with_global_state(r"\renewcommand{\zz}{\mathbb{Q}}", MathDisplay::Inline)
        .unwrap();
    let latex = r"\zz \oldzz";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("let_across_snippets", mathml.mathml, latex);
}

#[test]
fn test_let_without_global_group() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(r"\let\a\alpha\a", MathDisplay::Inline)
        .unwrap();
    // Outside of the global group, the binding is forgotten with the snippet.
    assert!(
        converter
            .convert_with_global_state(r"\a", MathDisplay::Inline)
            .is_err()
    );
}

/// `\global\let` writes into the store which outlives the snippet, even though the conversion
/// doesn't run in the global group.
#[test]
fn test_global_let() {
    // Each case defines something in the first snippet and uses it in the second one.
    for (name, definition, usage) in [
        // The source is a command defined in this very snippet, so its body has to be copied
        // into the store which outlives the snippet.
        (
            "global_let_to_local_cmd",
            r"\newcommand{\zzz}{z}\global\let\zzzz\zzz",
            r"\zzzz",
        ),
        // The copy has to follow a chain of `\let`s, and the arity has to survive it.
        (
            "global_let_chain",
            r"\newcommand{\m}[1]{\sqrt{#1}}\let\n\m\global\let\o\n",
            r"\o{2}",
        ),
        // Sources which live somewhere that outlives the snippet already, so nothing is copied.
        ("global_let_to_builtin", r"\global\let\a\alpha", r"\a"),
        (
            "global_let_to_builtin_with_args",
            r"\global\let\f\frac",
            r"\f{1}{2}",
        ),
        ("global_let_to_config_cmd", r"\global\let\c\cfgcmd", r"\c"),
        // `\global\global\let` is allowed, as it is in LaTeX.
        ("global_let_doubled", r"\global\global\let\a\alpha", r"\a"),
    ] {
        let config = MathCoreConfig {
            macros: vec![("cfgcmd".to_string(), r"\mathbb{C}".to_string())],
            pretty_print: PrettyPrint::Always,
            ..Default::default()
        };
        let mut converter = LatexToMathML::new(config).unwrap();
        converter
            .convert_with_global_state(definition, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{definition}` with error '{e}'"));
        let mathml = converter
            .convert_with_global_state(usage, MathDisplay::Inline)
            .unwrap_or_else(|e| panic!("failed to convert `{usage}` with error '{e}'"));
        assert_snapshot!(name, mathml.mathml, &format!("{definition}\n{usage}"));
    }
}

/// A `\global\let` copies the meaning of its source, but the *names* that meaning refers to are
/// still looked up when the command is expanded, as they are in LaTeX.
#[test]
fn test_global_let_keeps_late_binding() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(
            r"\newcommand{\inner}{x}\newcommand{\outer}{\inner+1}\global\let\g\outer",
            MathDisplay::Inline,
        )
        .unwrap();
    // `\g` outlived the snippet, but the `\inner` its body mentions did not, so expanding it
    // in the next snippet fails just like `\outer` itself would in LaTeX.
    assert!(
        converter
            .convert_with_global_state(r"\g", MathDisplay::Inline)
            .is_err()
    );
    // With `\inner` defined again, `\g` works.
    let latex = r"\newcommand{\inner}{y}\g";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("global_let_late_binding", mathml.mathml, latex);
}

/// A global definition takes effect right away, as it does in TeX: it is not shadowed by a
/// local definition of the same name for the rest of the snippet.
#[test]
fn test_global_let_beats_local() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    let latex = r"\newcommand{\a}{1}\a\global\let\a\alpha\a";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("global_let_beats_local", mathml.mathml, latex);
    // And it is still α in the next snippet.
    let latex = r"\a";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("global_let_beats_local_next_snippet", mathml.mathml, latex);
}

/// In the global group, every definition outlives the snippet anyway, so `\global` changes
/// nothing.
#[test]
fn test_global_let_in_global_group() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        global_group: true,
        ..Default::default()
    };
    let mut converter = LatexToMathML::new(config).unwrap();

    converter
        .convert_with_global_state(
            r"\newcommand{\zzz}{z}\global\let\zzzz\zzz",
            MathDisplay::Inline,
        )
        .unwrap();
    let latex = r"\zzz\zzzz";
    let mathml = converter
        .convert_with_global_state(latex, MathDisplay::Inline)
        .unwrap();
    assert_snapshot!("global_let_in_global_group", mathml.mathml, latex);
}

#[test]
fn test_let_with_ignore_unknown_commands() {
    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ignore_unknown_commands: true,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();

    // A `\let` to a name which is unknown at the time of the `\let` is not an error, and
    // the name can be defined later.
    let latex = r"\let\a\zzzz\newcommand{\zzzz}{x}\a";
    assert!(
        converter
            .convert_with_local_state(latex, MathDisplay::Inline)
            .is_err()
    );
}
