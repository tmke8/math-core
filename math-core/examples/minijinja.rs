use std::sync::Mutex;

use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};
use minijinja::value::{Object, Value};
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

#[derive(Debug)]
struct Converter {
    inner: Mutex<LatexToMathML>,
}

impl Converter {
    fn load(config: &MathCoreConfig) -> Converter {
        eprintln!("[info] loading converter");
        let converter = LatexToMathML::new(config).unwrap();
        eprintln!("[info] converter loaded");
        Converter {
            inner: Mutex::new(converter),
        }
    }
}

impl Object for Converter {}

fn mathml(state: &State, latex: &str) -> Option<Value> {
    let cache_key = "latex_to_mathml_converter";
    let converter = state.get_or_set_temp_object(cache_key, || {
        let config = state.lookup("mathcore_config").unwrap();
        let config = MathCoreConfig::deserialize(config).unwrap();
        eprintln!("[info] Config: {:?}", config);
        Converter::load(&config)
    });
    converter
        .inner
        .lock()
        .map_err(|_| eprintln!("[error] couldn't get lock for converter"))
        .ok()
        .and_then(|mut c| {
            c.convert_with_global_counter(latex, MathDisplay::Block)
                .map(|mathml| Value::from(mathml))
                .ok()
        })
}

fn main() {
    let mut env = Environment::new();
    env.add_function("mathml", mathml);
    let template = env.template_from_str(TEMPLATE).unwrap();
    let rv = template
        .render(context! {
            mathcore_config => MathCoreConfig {
                pretty_print: PrettyPrint::Always,
                ..Default::default()
            },
        })
        .unwrap();
    println!("{}", rv);
}
