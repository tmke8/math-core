use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

fn main() {
    let inputs = vec![
        // Safari bugs:
        r#"\bar x, \hat x, \check x, \grave x, \breve x, \acute x"#,
        r#"\vec x, \mathring x, \tilde x, \dot x, \ddot x, \dddot x, \ddddot x"#,
        r#"x^*"#,
        // Chrome bugs:
        r#"\widehat x, \widehat{xxxx}"#,
        r#"\widetilde x, \widetilde{xxxx}"#,
        r#"\widecheck x, \widecheck{xxxx}"#,
        r#"\overline x, \overline{xxxx}"#,
        r#"\underline x, \underline{xxxx}"#,
        r#"\begin{align*} ={}&x \end{align*}"#,
        r#"\Big\|x\Big\|_2"#,
        r#"x\left(y\right)z"#,
        // Firefox bugs:
        r#"{\int} x, \int x"#,
        r#"\begin{vmatrix} 1\\ 2 \end{vmatrix}"#,
    ];

    let converter = LatexToMathML::new(MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        allow_unreliable_rendering: true,
        ..Default::default()
    })
    .unwrap();
    let outputs = inputs
        .iter()
        .map(|input| {
            format!(
                "<code>{}</code><p>\n{}\n</p>",
                input,
                converter
                    .convert_with_local_counter(input, MathDisplay::Block)
                    .expect(input)
            )
        })
        .collect::<Vec<_>>()
        .join("</div>\n<div>");

    println!(
        r#"<!DOCTYPE html><html lang="en">
    <meta charset="UTF-8">
    <link rel="stylesheet" href="./mathmlfixes.css" />
    <style>
        @font-face {{
            font-family: "NewComputerModernMath Book";
            src: url('./fonts/NewCMMath-Book-prime-roundhand-vec-subset.woff2') format('woff2');
            font-display: swap;
        }}
        @font-face {{
            font-family: "NewComputerModern Book";
            src: url("./fonts/NewCM10-Book.woff2") format("woff2");
            font-display: swap;
        }}
        @font-face {{
            font-family: "NewComputerModern Mono";
            src: url("./fonts/NewCMMono10-Book.woff2") format("woff2");
            font-display: swap;
        }}
        math {{
            font-family: "NewComputerModernMath Book", math;
            mtext {{
                font-family: "NewComputerModern Book", serif;
                code {{
                    font-family: "NewComputerModern Mono", monospace;
                }}
            }}
        }}
    </style>
<body>
    <div>{}</div>
</body></html>"#,
        outputs
    );
}
