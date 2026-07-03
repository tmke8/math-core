use crate::ast::{IndentKeyword, Indentation};

pub fn new_line_and_indent(s: &mut String, indent_num: usize, indentation: Indentation) {
    if indent_num > 0 {
        s.push('\n');
    }
    for _ in 0..indent_num {
        match indentation {
            Indentation::Spaces(n) => {
                for _ in 0..n {
                    s.push(' ');
                }
            }
            Indentation::Keyword(IndentKeyword::Tab) => s.push('\t'),
        }
    }
}
