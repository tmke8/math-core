use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};
use minijinja::value::{Serde, Value};
use minijinja::{Environment, State, context};
use serde::Deserialize;

static TEMPLATE: &str = r#"
First equation:
{{ mathml('\\begin{align}
x
\\end{align}') }}

Second equation:
{{ mathml('\\begin{align}
y
\\end{align}') }}
"#;

struct Converter {
    inner: LatexToMathML,
}

impl Converter {
    fn load(config: MathCoreConfig) -> Converter {
        eprintln!("[info] loading converter");
        let converter = LatexToMathML::new(config, vec![]).unwrap();
        eprintln!("[info] converter loaded");
        Converter { inner: converter }
    }
}

fn mathml(state: &mut State, latex: &str) -> Option<Value> {
    let config = state.lookup("mathcore_config");
    let converter = state.get_or_insert_extension_with(|| {
        let config = MathCoreConfig::deserialize(config.unwrap()).unwrap();
        eprintln!("[info] Config: {:?}", config);
        Converter::load(config)
    });
    converter
        .inner
        .convert_with_global_state(latex, MathDisplay::Block)
        .map(|r| Value::from(r.mathml))
        .ok()
}

fn main() {
    let mut env = Environment::new();
    env.add_function("mathml", mathml);
    let template = env.template_from_str(TEMPLATE).unwrap();
    let rv = template
        .render(context! {
            mathcore_config => Serde(MathCoreConfig {
                pretty_print: PrettyPrint::Always,
                ..Default::default()
            }),
        })
        .unwrap();
    println!("{}", rv);
}
