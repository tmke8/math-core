use std::fmt;

use memchr::memmem::Finder;

use math_core::{Display, LatexError};

use crate::html_entities::replace_html_entities;

#[derive(Debug)]
pub struct ConversionError<'source, 'buf>(usize, ConvErrKind<'buf>, &'source str);

#[derive(Debug)]
pub enum ConvErrKind<'buf> {
    UnclosedDelimiter,
    NestedDelimiters,
    MismatchedDelimiters(usize),
    LatexError(LatexError<'buf>, &'buf str),
}

impl fmt::Display for ConversionError<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (line, col) = line_and_col(self.0, self.2);
        match &self.1 {
            ConvErrKind::UnclosedDelimiter => {
                write!(f, "Unclosed delimiter on line {line}, column {col}.")
            }
            ConvErrKind::NestedDelimiters => {
                write!(
                    f,
                    "Nested delimiters are not allowed (on line {line}, column {col})."
                )
            }
            ConvErrKind::MismatchedDelimiters(close) => {
                let (close_line, close_col) = line_and_col(*close, self.2);
                write!(
                    f,
                    "Mismatched delimiters: opening at line {line}, column {col}, closing at line {close_line}, column {close_col}."
                )
            }
            ConvErrKind::LatexError(e, input) => {
                // write!(f, "Error at {} in '{}':\n{}", idx, input, e)
                write!(
                    f,
                    "Error at line {line}, column {col} in '{}':\n{}",
                    input, e
                )
            }
        }
    }
}
impl std::error::Error for ConversionError<'_, '_> {}

/// Determine line and column numbers of `loc` within the input string.
fn line_and_col(loc: usize, input: &str) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in input.char_indices() {
        if i >= loc {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub struct Replacer<'args> {
    opening_finders: (Finder<'args>, Finder<'args>),
    closing_finders: (Finder<'args>, Finder<'args>),
    opening_lengths: (usize, usize),
    closing_lengths: (usize, usize),
    closing_identical: bool,
    /// A buffer for storing the result of replacing HTML entities.
    entity_buffer: String,
    ignore_escaped_delim: bool,
    continue_on_error: bool,
}

impl<'args> Replacer<'args> {
    pub fn new(
        inline_delim: (&'args str, &'args str),
        block_delim: (&'args str, &'args str),
        ignore_escaped_delim: bool,
        continue_on_error: bool,
    ) -> Self {
        let inline_opening = Finder::new(inline_delim.0);
        let inline_closing = Finder::new(inline_delim.1);
        let block_opening = Finder::new(block_delim.0);
        let block_closing = Finder::new(block_delim.1);

        Self {
            opening_finders: (inline_opening, block_opening),
            closing_finders: (inline_closing, block_closing),
            opening_lengths: (inline_delim.0.len(), block_delim.0.len()),
            closing_lengths: (inline_delim.1.len(), block_delim.1.len()),
            closing_identical: inline_delim.1 == block_delim.1,
            entity_buffer: String::new(),
            ignore_escaped_delim,
            continue_on_error,
        }
    }

    /// Replaces the content of inline and block math delimiters in a LaTeX string.
    ///
    /// Any kind of nesting of delimiters is not allowed.
    #[inline(always)]
    pub(crate) fn replace<'source, 'buf, F, S>(
        &'buf mut self,
        input: &'source str,
        state: &'buf mut S,
        f: F,
    ) -> Result<String, ConversionError<'source, 'buf>>
    where
        F: for<'a> Fn(&'a mut S, &mut String, &'a str, Display) -> Result<(), LatexError<'a>>,
        'source: 'buf,
    {
        let mut result = String::with_capacity(input.len());
        let mut current_pos = 0;

        while current_pos < input.len() {
            let remaining = &input[current_pos..];

            // Find the next occurrence of any opening delimiter
            let opening = self.find_next_delimiter(remaining, true);

            let Some((open_typ, idx)) = opening else {
                // No more opening delimiters found
                result.push_str(remaining);
                break;
            };

            let opening_delim_len = match open_typ {
                Display::Inline => self.opening_lengths.0,
                Display::Block => self.opening_lengths.1,
            };

            let open_pos = current_pos + idx;
            // Append everything before the opening delimiter
            result.push_str(&input[current_pos..open_pos]);
            // Skip the opening delimiter itself
            let start = open_pos + opening_delim_len;
            let remaining = &input[start..];

            // Find the next occurrence of any closing delimiter
            let closing = self.find_next_delimiter(remaining, false);

            let Some((close_typ, idx)) = closing else {
                // No closing delimiter found
                return Err(ConversionError(
                    open_pos,
                    ConvErrKind::UnclosedDelimiter,
                    input,
                ));
            };

            let closing_delim_len = match close_typ {
                Display::Inline => self.closing_lengths.0,
                Display::Block => self.closing_lengths.1,
            };

            if !self.closing_identical && open_typ != close_typ {
                // Mismatch of opening and closing delimiter
                return Err(ConversionError(
                    open_pos,
                    ConvErrKind::MismatchedDelimiters(start + idx),
                    input,
                ));
            }

            let end = start + idx;
            // Get the content between delimiters
            let content = &input[start..end];
            // Check whether any *opening* delimiters are present in the content
            if let Some((_, idx)) = self.find_next_delimiter(content, true) {
                return Err(ConversionError(
                    start + idx,
                    ConvErrKind::NestedDelimiters,
                    input,
                ));
            }
            // Replace HTML entities
            let replaced = replace_html_entities(&mut self.entity_buffer, content);
            // Convert the content and check for error.
            let is_error = { f(state, &mut result, replaced, open_typ).is_err() };
            if is_error {
                if self.continue_on_error {
                    // If we continue on error, we just skip the conversion and return the
                    // original content (including delimiters).
                    result.push_str(&input[open_pos..end + closing_delim_len]);
                } else {
                    // If we stop on error, return the error together with the snippet.
                    // Unfortunately, due to limitations in the borrow checker, we have to run the
                    // conversion again to get the error.
                    // The reason seems to be that the borrow checker cannot tell that when we
                    // return `replaced` here, we are not maintaining the borrow for the next
                    // iteration of the loop.
                    // This is quite unfortunate, but we only have to do this in the error case,
                    // which is hopefully not too common.
                    let replaced = replace_html_entities(&mut self.entity_buffer, content);
                    let latex_error = f(state, &mut result, replaced, open_typ).unwrap_err();
                    return Err(ConversionError(
                        start,
                        ConvErrKind::LatexError(latex_error, replaced),
                        input,
                    ));
                }
            }
            // Update current position
            current_pos = end + closing_delim_len;
        }

        Ok(result)
    }

    /// Finds the next occurrence of either an inline or block delimiter.
    fn find_next_delimiter(&self, input: &str, opening: bool) -> Option<(Display, usize)> {
        let input = input.as_bytes();

        // Find positions for both delimiter types
        let inline_result = self.find_delimiter_position(
            input,
            if opening {
                &self.opening_finders.0
            } else {
                &self.closing_finders.0
            },
            if opening {
                self.opening_lengths.0
            } else {
                self.closing_lengths.0
            },
        );

        let block_result = self.find_delimiter_position(
            input,
            if opening {
                &self.opening_finders.1
            } else {
                &self.closing_finders.1
            },
            if opening {
                self.opening_lengths.1
            } else {
                self.closing_lengths.1
            },
        );

        // Return the closest delimiter, with display taking priority on ties
        match (inline_result, block_result) {
            (Some(inline_pos), Some(block_pos)) => {
                if block_pos <= inline_pos {
                    Some((Display::Block, block_pos))
                } else {
                    Some((Display::Inline, inline_pos))
                }
            }
            (Some(pos), None) => Some((Display::Inline, pos)),
            (None, Some(pos)) => Some((Display::Block, pos)),
            (None, None) => None,
        }
    }

    /// Helper function to find the next unescaped delimiter position
    fn find_delimiter_position(
        &self,
        input: &[u8],
        finder: &Finder,
        delimiter_len: usize,
    ) -> Option<usize> {
        if !self.ignore_escaped_delim {
            return finder.find(input);
        }

        let mut offset = 0;

        while let Some(relative_pos) = finder.find(&input[offset..]) {
            let absolute_pos = offset + relative_pos;

            // Check if this delimiter is escaped
            if absolute_pos > 0 && input[absolute_pos - 1] == b'\\' {
                // Skip past this escaped delimiter
                offset = absolute_pos + delimiter_len;
                continue;
            }

            return Some(absolute_pos);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_core::LatexErrKind;
    use std::fmt::Write;

    /// Mock convert function for testing
    fn mock_convert<'source>(
        _state: &mut (),
        buf: &mut String,
        content: &'source str,
        typ: Display,
    ) -> Result<(), LatexError<'source>> {
        match typ {
            Display::Inline => write!(buf, "[T1:{}]", content).unwrap(),
            Display::Block => write!(buf, "[T2:{}]", content).unwrap(),
        };
        Ok(())
    }

    fn replace(
        input: &'static str,
        inline_delim: (&str, &str),
        block_delim: (&str, &str),
        ignore_escaped_delim: bool,
    ) -> Result<String, ConversionError<'static, 'static>> {
        let mut replacer = Replacer::new(inline_delim, block_delim, ignore_escaped_delim, false);
        match replacer.replace(input, &mut (), mock_convert) {
            Ok(s) => Ok(s),
            Err(e) => match &e.1 {
                // The following is needed to do a kind of "lifetime laundering".
                ConvErrKind::MismatchedDelimiters(close) => Err(ConversionError(
                    e.0,
                    ConvErrKind::MismatchedDelimiters(*close),
                    input,
                )),
                ConvErrKind::NestedDelimiters => {
                    Err(ConversionError(e.0, ConvErrKind::NestedDelimiters, input))
                }
                ConvErrKind::UnclosedDelimiter => {
                    Err(ConversionError(e.0, ConvErrKind::UnclosedDelimiter, input))
                }
                ConvErrKind::LatexError(_, _) => unreachable!(),
            },
        }
    }

    #[test]
    fn test_basic_replacement() {
        let input = "Hello $world$ and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "Hello [T1:world] and [T2:universe]");
    }

    #[test]
    fn test_escaping_single() {
        let input = "Hello\\$ world and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), true).unwrap();
        assert_eq!(result, "Hello\\$ world and [T2:universe]");
    }

    #[test]
    fn test_escaping_single_inline_delim() {
        let input = "Hello\\$ $world$ and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), true).unwrap();
        assert_eq!(result, "Hello\\$ [T1:world] and [T2:universe]");
    }

    #[test]
    fn test_escaping_double() {
        let input = "Hello \\$world\\$ and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), true).unwrap();
        assert_eq!(result, "Hello \\$world\\$ and [T2:universe]");
    }

    #[test]
    fn test_escaping_block() {
        let input = "Hello \\(world\\) and \\$$universe";
        let result = replace(input, ("\\(", "\\)"), ("$$", "$$"), true).unwrap();
        assert_eq!(result, "Hello [T1:world] and \\$$universe");
    }

    #[test]
    fn test_escaping_block_double() {
        let input = "Hello \\(world\\) and \\$$universe\\$$";
        let result = replace(input, ("\\(", "\\)"), ("$$", "$$"), true).unwrap();
        assert_eq!(result, "Hello [T1:world] and \\$$universe\\$$");
    }

    #[test]
    fn test_nested_delimiters() {
        let input = "Nested $$outer $inner$ delimiter$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(7, ConvErrKind::MismatchedDelimiters(15), _)
        ));
    }

    #[test]
    fn test_nested_delimiters2() {
        let input = "Nested $outer $$inner$$ delimiter$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(7, ConvErrKind::MismatchedDelimiters(14), _)
        ));
    }

    #[test]
    fn test_mismatched_unclosed() {
        let input = "Unclosed $delimiter";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(9, ConvErrKind::UnclosedDelimiter, _)
        ));
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_delimiters() {
        let input = "Hello, world!";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_multiple_replacements() {
        let input = "$a$ then $$b$$ then $c$ and $$d$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "[T1:a] then [T2:b] then [T1:c] and [T2:d]");
    }

    #[test]
    fn test_complete_replacements() {
        let input = "$a then b then c and d$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "[T1:a then b then c and d]");
    }

    #[test]
    fn test_mismatched_delimiters() {
        let input = "Mismatch $$ and $ signs";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(9, ConvErrKind::MismatchedDelimiters(16), _)
        ));
    }

    #[test]
    fn test_identical_delimiters() {
        let input = "|a| and ||b||";
        let result = replace(input, ("|", "|"), ("||", "||"), false).unwrap();
        assert_eq!(result, "[T1:a] and [T2:b]");
    }

    #[test]
    fn test_asymmetric_delimiters() {
        let input = r"let \(a=1\) and \[b=2\].";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap();
        assert_eq!(result, "let [T1:a=1] and [T2:b=2].");
    }

    #[test]
    fn test_asymmetric_delimiters_partial_delim() {
        let input = r"let\ \(a=1\) and \[b=2\].";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap();
        assert_eq!(result, "let\\ [T1:a=1] and [T2:b=2].");
    }

    #[test]
    fn test_asymmetric_delimiters_nested() {
        let input = r"let \(a=1 and \[b=2\]\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(4, ConvErrKind::MismatchedDelimiters(19), _)
        ));
    }

    #[test]
    fn test_asymmetric_delimiters_nested2() {
        let input = r"let \(a=1 and \[b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(14, ConvErrKind::NestedDelimiters, _)
        ));
    }

    #[test]
    fn test_asymmetric_delimiters_nested3() {
        let input = r"let \(a=1 and \(b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(14, ConvErrKind::NestedDelimiters, _)
        ));
    }

    #[test]
    fn test_asymmetric_delimiters_unclosed() {
        let input = r"let \(a=1 and b=2.";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        assert!(matches!(
            result,
            ConversionError(4, ConvErrKind::UnclosedDelimiter, _)
        ));
    }

    #[test]
    fn test_asymmetric_delimiters_dangling() {
        // We could make this an error, but it's sometimes useful to allow this.
        let input = r"let a=1\) and \(b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap();
        assert_eq!(result, r"let a=1\) and [T1:b=2].");
    }

    #[test]
    fn test_asymmetric_delimiters_dangling2() {
        // We could make this an error, but it's sometimes useful to allow this.
        let input = r"let \(a=1\) and b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap();
        assert_eq!(result, r"let [T1:a=1] and b=2\).");
    }

    #[test]
    fn test_multibyte_delimiters() {
        let input = "this is über ü(a=2ü).";
        let result = replace(input, ("ü(", "ü)"), ("ü[", "ü]"), false).unwrap();
        assert_eq!(result, "this is über [T1:a=2].");
    }

    #[test]
    fn test_long_delimiters() {
        let input = r#"based on its length, <span class="math inline">P(p)=2^{-len(p)}</span>, and then for a given
    <span class="math block">
    P(p)=2^{-len(p)}
    </span>
    Hello."#;
        let result = replace(
            input,
            ("<span class=\"math inline\">", "</span>"),
            ("<span class=\"math block\">", "</span>"),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            "based on its length, [T1:P(p)=2^{-len(p)}], and then for a given\n    [T2:\n    P(p)=2^{-len(p)}\n    ]\n    Hello."
        );
    }

    #[test]
    fn test_error() {
        let mut replacer = Replacer::new((r"\(", r"\)"), (r"\[", r"\]"), false, false);
        let input = r"let \(&amp;=1\).";
        let mut unit = ();
        // This conversion function always returns an error.
        let err = replacer
            .replace(input, &mut unit, |_state, _buf, _content, _typ| {
                Err(LatexError(0, LatexErrKind::UnexpectedEOF))
            })
            .unwrap_err();
        assert!(matches!(
            err,
            ConversionError(
                6,
                ConvErrKind::LatexError(LatexError(0, LatexErrKind::UnexpectedEOF), "&=1"),
                _
            )
        ));
    }

    #[test]
    fn test_error_multiline() {
        let mut replacer = Replacer::new((r"\(", r"\)"), (r"\[", r"\]"), false, false);
        let input = "hello world\nlet\\(&amp;=1\\).";
        let mut unit = ();
        // This conversion function always returns an error.
        let err = replacer
            .replace(input, &mut unit, |_state, _buf, _content, _typ| {
                Err(LatexError(0, LatexErrKind::UnexpectedEOF))
            })
            .unwrap_err();
        assert!(format!("{err}").contains("line 2, column 6"));
    }
}
