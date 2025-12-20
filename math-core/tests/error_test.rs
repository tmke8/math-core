use insta::assert_snapshot;
use math_core::{LatexError, LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

#[test]
fn main() {
    let problems = [
        ("end_without_open", r"\end{matrix}"),
        ("curly_close_without_open", r"}"),
        ("unsupported_command", r"\asdf"),
        (
            "unsupported_environment",
            r"\begin{xmatrix} 1 \end{xmatrix}",
        ),
        ("incorrect_bracket", r"\operatorname[lim}"),
        ("unclosed_bracket", r"\sqrt[lim"),
        ("mismatched_begin_end", r"\begin{matrix} 1 \end{bmatrix}"),
        (
            "spaces_in_env_name",
            r"\begin{  pmatrix   } x \\ y \end{pmatrix}",
        ),
        (
            "incorrect_bracket_in_begin",
            r"\begin{matrix] 1 \end{matrix}",
        ),
        ("incomplete_sup", r"x^"),
        ("invalid_sup", r"x^^"),
        ("invalid_sub_sup", r"x^_"),
        ("double_sub", r"x__3"),
        ("int_double_sub", r"\int__3 x dx"),
        ("unicode_command", r"\éx"),
        ("wrong_opening_paren", r"\begin[matrix} x \end{matrix}"),
        ("unclosed_brace", r"{"),
        ("unclosed_left", r"\left( x"),
        ("unclosed_env", r"\begin{matrix} x"),
        ("unclosed_begin", r"\begin{matrix"),
        ("unclosed_text", r"\text{hello"),
        ("unexpected_limits", r"\text{hello}\limits_0^1"),
        ("unsupported_not", r"\not\text{hello}"),
        ("text_with_unclosed_group", r"\text{x{}"),
        ("text_with_math_command", r"\text{\max}"),
        ("text_at_eof", r"\sum\text"),
        ("operatorname_with_end", r"\operatorname{\end{matrix}}"),
        ("operatorname_with_begin", r"\operatorname{\begin{matrix}}"),
        ("operatorname_with_text_command", r"\operatorname{\ae}"),
        ("super_then_prime", "f^2'"),
        ("sub_super_then_prime", "f_5^2'"),
        ("sup_sup", "x^2^3 y"),
        ("sub_sub", "x_2_3 y"),
        ("no_rbrack_instead_of_bracket", r"\sqrt[3\rbrack{1}"),
        ("genfrac_wrong_unit", r"\genfrac(]{1pg}{2}{a+b}{c+d}"),
        ("hspace_empty", r"\hspace{  }"),
        ("hspace_unknown_unit", r"\hspace{2ly}"),
        ("hspace_non_digits", r"\hspace{2b2cm}"),
        ("hspace_non_ascii", r"\hspace{22öm}"),
        ("ampersand_outside_array", r"x & y"),
        ("sqrt_unknown_cmd", r"\sqrt[3]\asdf 3"),
        ("mathrm_unknown_cmd", r"\mathrm{ab\asdf}"),
        ("digits_unknown_cmd", r"1.1\asdf"),
        ("tag_with_non_number", r"\begin{align}x\tag{A1}\end{align}"),
        ("tag_with_empty", r"\begin{align} x \tag{} \\ y \end{align}"),
        ("tag_with_zero", r"\begin{align} x \tag{0} \\ y \end{align}"),
        ("tag_in_aligned", r#"\begin{aligned}\tag{32}1\end{aligned}"#),
        (
            "ampersand_in_multline",
            r#"\begin{multline}1&1\end{multline}"#,
        ),
        ("ampersand_in_gather", r#"\begin{gather}1&1\\1\end{gather}"#),
    ];

    let config = MathCoreConfig {
        pretty_print: PrettyPrint::Never,
        ..Default::default()
    };
    let converter = LatexToMathML::new(config).unwrap();
    for (name, problem) in problems.into_iter() {
        let Err(LatexError(loc, error)) = converter
            .convert_with_local_counter(problem, MathDisplay::Inline)
            .map_err(|e| *e)
        else {
            panic!("problem `{}` did not return an error", problem);
        };
        let output = format!("Position: {}\n{:#?}", loc, error);
        assert_snapshot!(name, &output, problem);
    }
}
