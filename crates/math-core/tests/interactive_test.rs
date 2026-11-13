use math_core::{
    ConvertResult, InteractiveDocument, LatexError, LatexToMathML, MathCoreConfig, MathDisplay,
    TaggingStrategy,
};

use MathDisplay::{Block, Inline};

fn equation(body: &str) -> String {
    format!(r"\begin{{equation}}{body}\end{{equation}}")
}

fn block(latex: impl Into<String>) -> (String, MathDisplay) {
    (latex.into(), Block)
}

fn inline(latex: impl Into<String>) -> (String, MathDisplay) {
    (latex.into(), Inline)
}

fn new_document() -> InteractiveDocument {
    InteractiveDocument::new(MathCoreConfig::default(), vec![]).unwrap()
}

/// The output of a snippet in a form which can be compared.
fn comparable(result: &Result<ConvertResult, Box<LatexError>>) -> String {
    match result {
        Ok(result) => result.mathml.clone(),
        Err(err) => format!("error at {:?}: {}", err.0, err.error_message()),
    }
}

fn outputs(doc: &InteractiveDocument) -> Vec<String> {
    (0..doc.len())
        .map(|index| comparable(doc.get(index).unwrap()))
        .collect()
}

fn mathml(doc: &InteractiveDocument, index: usize) -> &str {
    &doc.get(index).unwrap().as_ref().unwrap().mathml
}

#[test]
fn numbers_shift_when_an_equation_is_inserted() {
    let mut doc = new_document();
    let changed = doc.splice(
        0..0,
        [
            inline("x"),
            block(equation("a")),
            inline("y"),
            block(equation("b")),
            block("z"),
        ],
    );
    assert_eq!(changed, [0, 1, 2, 3, 4]);
    assert!(mathml(&doc, 1).contains("(1)"));
    assert!(mathml(&doc, 3).contains("(2)"));

    // Only the equations with a number are affected by another number in front of them.
    let changed = doc.insert(1, equation("c"), Block);
    assert_eq!(changed, [1, 2, 4]);
    assert!(mathml(&doc, 1).contains("(1)"));
    assert!(mathml(&doc, 2).contains("(2)"));
    assert!(mathml(&doc, 4).contains("(3)"));

    // An equation without a number affects nothing else.
    assert_eq!(doc.insert(0, "w".to_string(), Inline), [0]);
    assert_eq!(doc.replace(3, equation("a + 1"), Block), [3]);

    let changed = doc.remove(2);
    assert_eq!(changed, [2, 4]);
    assert!(mathml(&doc, 2).contains("(1)"));
    assert!(mathml(&doc, 4).contains("(2)"));
}

#[test]
fn references_follow_their_equation() {
    let mut doc = new_document();
    doc.splice(
        0..0,
        [
            // A forward reference.
            inline(r"\eqref{eq:b}"),
            block(equation(r"a \label{eq:a}")),
            inline("x"),
            block(equation(r"b \label{eq:b}")),
            inline(r"\eqref{eq:a}"),
        ],
    );
    assert!(mathml(&doc, 0).contains("(2)"));
    assert!(mathml(&doc, 4).contains("(1)"));

    // The second equation loses its number, which the reference in front of it has to show,
    // but the reference to the other equation doesn't.
    let changed = doc.replace(3, r"b \tag{*} \label{eq:b}".to_string(), Block);
    assert_eq!(changed, [0, 3]);
    assert!(mathml(&doc, 0).contains("(*)"));

    // The label disappears.
    let changed = doc.remove(3);
    assert_eq!(changed, [0]);
    assert!(mathml(&doc, 0).contains("(??)"));
    assert!(
        doc.get(0)
            .unwrap()
            .as_ref()
            .unwrap()
            .warnings
            .has_undefined_references()
    );

    // A label which gets a second definition, further down, refers to that one.
    let changed = doc.insert(4, r"c \tag{C} \label{eq:a}".to_string(), Block);
    assert_eq!(changed, [3, 4]);
    assert!(mathml(&doc, 3).contains("(C)"));
}

#[test]
fn global_definitions_reach_later_snippets() {
    let mut doc = new_document();
    doc.splice(
        0..0,
        [
            inline(r"\RR"),
            inline(r"\gdef\RR{\mathbb{R}}"),
            inline(r"\RR"),
            inline("x"),
            block(equation(r"\RR")),
        ],
    );
    // The definition comes too late for the first snippet.
    assert!(doc.get(0).unwrap().is_err());
    assert!(mathml(&doc, 2).contains("ℝ"));
    assert!(mathml(&doc, 4).contains("ℝ"));

    // All snippets after a definition are converted again when it changes, but only those
    // which use it come out differently.
    let changed = doc.replace(1, r"\gdef\RR{\mathbf{R}}".to_string(), Inline);
    assert_eq!(changed, [1, 2, 4]);
    assert!(mathml(&doc, 2).contains("𝐑"));

    // Editing a snippet without touching the definition in it affects nothing else.
    let changed = doc.replace(1, r"\gdef\RR{\mathbf{R}} y".to_string(), Inline);
    assert_eq!(changed, [1]);

    // Without the definition, the snippets which use it fail.
    let changed = doc.remove(1);
    assert_eq!(changed, [1, 3]);
    assert!(doc.get(1).unwrap().is_err());
    assert!(doc.get(3).unwrap().is_err());

    // A definition without `\global` stays in its snippet.
    let changed = doc.insert(0, r"\def\RR{\mathbb{R}} \RR".to_string(), Inline);
    assert_eq!(changed, [0]);
}

#[test]
fn several_snippets_at_once() {
    let mut doc = new_document();
    doc.splice(
        0..0,
        [
            block(equation("a")),
            block(equation("b")),
            block(equation("c")),
            block(equation(r"d \label{eq:d}")),
            inline(r"\eqref{eq:d}"),
        ],
    );
    // Two numbered equations are replaced by one equation with two numbers and a formula
    // without a number, so the numbers after them stay as they are.
    let changed = doc.splice(
        1..3,
        [
            block(r"\begin{align} b \\ c \end{align}"),
            inline("x"),
            inline("y"),
        ],
    );
    assert_eq!(changed, [1, 2, 3]);
    assert_eq!(doc.len(), 6);
    assert!(mathml(&doc, 5).contains("(4)"));
    assert_eq!(doc.source(2), Some(("x", Inline)));

    let changed = doc.splice(0..2, []);
    assert_eq!(changed, [2, 3]);
    assert!(mathml(&doc, 3).contains("(1)"));

    doc.clear();
    assert!(doc.is_empty());
    assert!(doc.get(0).is_none());
    // The equation count starts from the beginning again.
    assert_eq!(doc.insert(0, equation("a"), Block), [0]);
    assert!(mathml(&doc, 0).contains("(1)"));
}

#[test]
fn failed_snippets() {
    let mut doc = new_document();
    doc.splice(
        0..0,
        [
            block(equation("a")),
            block(equation(r"\frac{b}")),
            block(equation("c")),
        ],
    );
    assert!(doc.get(1).unwrap().is_err());
    assert!(mathml(&doc, 2).contains("(2)"));

    let changed = doc.replace(1, equation(r"\frac{b}{2}"), Block);
    assert_eq!(changed, [1, 2]);
    assert!(mathml(&doc, 2).contains("(3)"));
}

#[test]
#[should_panic(expected = "snippet range out of bounds")]
fn splice_out_of_bounds() {
    let mut doc = new_document();
    doc.insert(1, "x".to_string(), Inline);
}

#[test]
#[should_panic(expected = "snippet index out of bounds")]
fn remove_out_of_bounds() {
    let mut doc = new_document();
    doc.remove(0);
}

/// A small pseudo-random number generator (xorshift), so that the test below is reproducible
/// without any dependencies.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }
}

/// Snippets which depend on each other in all the ways there are.
const POOL: &[(&str, MathDisplay)] = &[
    ("x + y", Inline),
    (r"\frac{1}{2}", Block),
    (r"\begin{equation} a = b \end{equation}", Block),
    (r"\begin{equation} a = b \label{eq:a} \end{equation}", Block),
    (r"\begin{equation} c = d \label{eq:b} \end{equation}", Block),
    (r"\begin{equation*} c = d \end{equation*}", Block),
    (
        r"\begin{align} a &= b \label{eq:a} \\ c &= d \notag \\ e &= f \label{eq:c} \end{align}",
        Block,
    ),
    (r"e = f \tag{*} \label{eq:b}", Block),
    (r"g = h \tag{A} \label{eq:c}", Block),
    (r"\eqref{eq:a}", Inline),
    (r"\eqref{eq:b} + \eqref{eq:c}", Inline),
    (r"\eqref{eq:d}", Block),
    (r"\gdef\RR{\mathbb{R}}", Inline),
    (r"\gdef\RR{\mathbf{R}} x \in \RR", Inline),
    (r"\gdef\N{\RR^+}", Inline),
    (r"\global\let\S=\RR", Inline),
    (r"\def\RR{r} \RR", Inline),
    (r"\newcommand{\RR}{\mathcal{R}} \RR", Inline),
    (r"\renewcommand{\RR}{\mathfrak{R}} \RR", Inline),
    (r"\providecommand{\N}{n} \N", Inline),
    (r"\RR", Inline),
    (r"\N + \S", Inline),
    // A definition which shadows a built-in command, and a use of that command.
    (r"\gdef\R{\mathrm{R}}", Inline),
    (r"\R", Inline),
    (r"\begin{equation} \RR \label{eq:b} \end{equation}", Block),
    // Snippets which fail, some of them after they have had an effect on the state.
    (r"\frac{1}", Inline),
    (
        r"\begin{align} a \label{eq:a} \\ b \label{eq:c} \\ \frac{ \end{align}",
        Block,
    ),
    (r"\gdef\N{n} \frac{1}", Inline),
    (r"\begin{align} a \end{align}", Inline),
];

fn random_snippets(rng: &mut Rng, max: usize) -> Vec<(String, MathDisplay)> {
    (0..rng.below(max + 1))
        .map(|_| {
            let (latex, display) = POOL[rng.below(POOL.len())];
            (latex.to_string(), display)
        })
        .collect()
}

/// Edit a document at random, and check after every edit that the outputs are the ones of a
/// conversion from scratch and that every snippet with a new output has been reported.
fn check_random_edits(config: &MathCoreConfig, seed: u64) {
    let converter = LatexToMathML::new(config.clone(), vec![]).unwrap();
    let mut doc = InteractiveDocument::new(config.clone(), vec![]).unwrap();
    let mut rng = Rng(seed);
    let mut sources: Vec<(String, MathDisplay)> = Vec::new();

    for step in 0..400 {
        let start = rng.below(sources.len() + 1);
        let end = start + rng.below((sources.len() - start).min(3) + 1);
        let new_snippets = if step % 50 == 49 {
            // Clear the document every now and then, so that it doesn't just grow.
            sources.clear();
            doc.clear();
            continue;
        } else {
            random_snippets(&mut rng, 3)
        };

        // What the outputs are expected to be if the edit affects no other snippet.
        let mut expected: Vec<Option<String>> = outputs(&doc).into_iter().map(Some).collect();
        expected.splice(start..end, new_snippets.iter().map(|_| None));

        sources.splice(start..end, new_snippets.iter().cloned());
        let changed = doc.splice(start..end, new_snippets);

        assert_eq!(doc.len(), sources.len());
        assert!(changed.is_sorted_by(|a, b| a < b), "unsorted: {changed:?}");

        let from_scratch = converter.convert_all(&sources);
        for (index, reference) in from_scratch.iter().enumerate() {
            let context = format!("step {step}, snippet {index} of {sources:#?}");
            let output = comparable(doc.get(index).unwrap());
            assert_eq!(output, comparable(reference), "{context}");
            match &expected[index] {
                // A new snippet is always reported.
                None => assert!(changed.contains(&index), "{context}"),
                Some(old) if *old != output => {
                    assert!(changed.contains(&index), "not reported: {context}");
                }
                // An output which is still the same is only ever reported for a snippet
                // which fails.
                Some(_) if changed.contains(&index) => {
                    assert!(reference.is_err(), "reported without reason: {context}");
                }
                Some(_) => {}
            }
        }
    }
}

#[test]
fn random_edits_match_conversion_from_scratch() {
    for seed in 1..=20 {
        check_random_edits(&MathCoreConfig::default(), seed);
    }
}

#[test]
fn random_edits_in_global_group() {
    let config = MathCoreConfig {
        global_group: true,
        tags: TaggingStrategy::All,
        annotation: true,
        ..Default::default()
    };
    for seed in 21..=40 {
        check_random_edits(&config, seed);
    }
}

#[test]
fn random_edits_without_numbers() {
    let config = MathCoreConfig {
        tags: TaggingStrategy::None,
        ignore_unknown_commands: true,
        ..Default::default()
    };
    for seed in 41..=50 {
        check_random_edits(&config, seed);
    }
}
