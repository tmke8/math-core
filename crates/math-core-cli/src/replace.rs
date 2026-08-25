use std::borrow::Cow;
use std::fmt;

use memchr::memmem::Finder;

use math_core::{LatexError, MathDisplay, Warnings};

use crate::html_entities::replace_html_entities;

#[derive(Debug)]
pub struct ConversionError<'source>(usize, ConvErrKind<'source>, &'source str);

#[derive(Debug)]
pub enum ConvErrKind<'source> {
    UnclosedDelimiter,
    NestedDelimiters,
    MismatchedDelimiters(usize),
    /// A snippet failed to convert; carries the error and the snippet it occurred in.
    LatexError(LatexError, Cow<'source, str>),
}

impl<'source> ConversionError<'source> {
    /// Report that the snippet found at `site` failed to convert.
    pub(crate) fn latex_error(
        input: &'source str,
        site: &Site,
        latex: Cow<'source, str>,
        error: LatexError,
    ) -> Self {
        ConversionError(site.offset, ConvErrKind::LatexError(error, latex), input)
    }
}

impl fmt::Display for ConversionError<'_> {
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
            ConvErrKind::LatexError(e, snippet) => {
                let source_name = "<input>";
                let report = e.to_report(source_name, crate::use_color());
                let mut buf = Vec::new();
                report
                    .write((source_name, ariadne::Source::from(snippet)), &mut buf)
                    .expect("failed to write report");
                f.write_str(std::str::from_utf8(&buf).expect("report should be valid UTF-8"))
            }
        }
    }
}
impl std::error::Error for ConversionError<'_> {}

/// A warning that the conversion of one snippet of a document produced.
///
/// Unlike a [`ConversionError`], a warning does not stop the conversion; it only points out that
/// the resulting MathML is probably not what the author intended.
#[derive(Debug)]
pub struct SnippetWarning<'source>(usize, WarnKind, &'source str);

#[derive(Debug)]
enum WarnKind {
    UndefinedReference,
    UnknownCommand,
}

impl<'source> SnippetWarning<'source> {
    /// The warnings that the snippet found at `site` produced, one per kind.
    pub(crate) fn for_site(
        input: &'source str,
        site: &Site,
        warnings: Warnings,
    ) -> impl Iterator<Item = Self> {
        let offset = site.offset;
        [
            warnings
                .has_undefined_references()
                .then_some(WarnKind::UndefinedReference),
            warnings
                .has_unknown_commands()
                .then_some(WarnKind::UnknownCommand),
        ]
        .into_iter()
        .flatten()
        .map(move |kind| SnippetWarning(offset, kind, input))
    }
}

impl fmt::Display for SnippetWarning<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (line, col) = line_and_col(self.0, self.2);
        let what = match self.1 {
            WarnKind::UndefinedReference => "undefined reference",
            WarnKind::UnknownCommand => "unknown command",
        };
        write!(f, "{what} in the formula on line {line}, column {col}.")
    }
}

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
    ignore_escaped_delim: bool,
}

/// Where a LaTeX snippet was found in the document.
pub(crate) struct Site<'source> {
    /// The document text between the previous snippet (or the start of the document) and this one.
    pub(crate) preceding_text: &'source str,
    /// The byte offset of the snippet's content within the document, for error reporting.
    pub(crate) offset: usize,
}

/// A document split into LaTeX snippets and the text surrounding them.
pub(crate) struct Scan<'source> {
    /// The snippets in document order, in the shape [`math_core::LatexToMathML::convert_all`]
    /// expects.
    pub(crate) snippets: Vec<(Cow<'source, str>, MathDisplay)>,
    /// Where each snippet came from; `sites[i]` belongs to `snippets[i]`.
    pub(crate) sites: Vec<Site<'source>>,
    /// The document text following the last snippet.
    pub(crate) trailing_text: &'source str,
}

impl<'args> Replacer<'args> {
    pub fn new(
        inline_delim: (&'args str, &'args str),
        block_delim: (&'args str, &'args str),
        ignore_escaped_delim: bool,
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
            ignore_escaped_delim,
        }
    }

    /// Finds all LaTeX snippets between inline and block math delimiters.
    ///
    /// The snippets are returned together with the surrounding document text, so that the
    /// document can be reassembled once the snippets have been converted. Conversion cannot
    /// happen during the scan, because [`math_core::LatexToMathML::convert_all`] needs to see all
    /// snippets of a document at once in order to resolve forward references.
    ///
    /// Any kind of nesting of delimiters is not allowed.
    pub(crate) fn scan<'source>(
        &self,
        input: &'source str,
    ) -> Result<Scan<'source>, ConversionError<'source>> {
        let mut snippets = Vec::new();
        let mut sites = Vec::new();
        let mut current_pos = 0;

        while current_pos < input.len() {
            let remaining = &input[current_pos..];

            // Find the next occurrence of any opening delimiter
            let opening = self.find_next_delimiter(remaining, true);

            let Some((open_typ, idx)) = opening else {
                // No more opening delimiters found
                break;
            };

            let opening_delim_len = match open_typ {
                MathDisplay::Inline => self.opening_lengths.0,
                MathDisplay::Block => self.opening_lengths.1,
            };

            let open_pos = current_pos + idx;
            // Everything before the opening delimiter is copied verbatim later on.
            let preceding_text = &input[current_pos..open_pos];
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
                MathDisplay::Inline => self.closing_lengths.0,
                MathDisplay::Block => self.closing_lengths.1,
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
            snippets.push((replace_html_entities(content), open_typ));
            sites.push(Site {
                preceding_text,
                offset: start,
            });
            // Update current position
            current_pos = end + closing_delim_len;
        }

        Ok(Scan {
            snippets,
            sites,
            trailing_text: &input[current_pos..],
        })
    }

    /// Finds the next occurrence of either an inline or block delimiter.
    ///
    /// Both delimiters are searched within a window that starts small and doubles until a
    /// delimiter is found or the whole input has been covered. This keeps the cost of a call
    /// proportional to the distance to the nearest delimiter instead of to the length of
    /// `input`. Searching the full input for both delimiters would be quadratic overall
    /// whenever one of the two never occurs in the document — e.g. the block delimiter in a
    /// document that only contains inline math, which then gets scanned to the end of the file
    /// once per formula.
    fn find_next_delimiter(&self, input: &str, opening: bool) -> Option<(MathDisplay, usize)> {
        /// Size of the first window; large enough that densely spaced delimiters are found on
        /// the first attempt.
        const INITIAL_WINDOW: usize = 1024;

        let input = input.as_bytes();
        let (inline_finder, block_finder) = if opening {
            (&self.opening_finders.0, &self.opening_finders.1)
        } else {
            (&self.closing_finders.0, &self.closing_finders.1)
        };
        let (inline_len, block_len) = if opening {
            self.opening_lengths
        } else {
            self.closing_lengths
        };

        let mut window = INITIAL_WINDOW;
        loop {
            let end = window.min(input.len());
            let haystack = &input[..end];

            let inline_result = self.find_delimiter_position(haystack, inline_finder, inline_len);
            let block_result = self.find_delimiter_position(haystack, block_finder, block_len);

            // Take the closest delimiter, with block display taking priority on ties.
            let found = match (inline_result, block_result) {
                (Some(inline_pos), Some(block_pos)) => {
                    if block_pos <= inline_pos {
                        (MathDisplay::Block, block_pos)
                    } else {
                        (MathDisplay::Inline, inline_pos)
                    }
                }
                (Some(pos), None) => (MathDisplay::Inline, pos),
                (None, Some(pos)) => (MathDisplay::Block, pos),
                (None, None) => {
                    if end == input.len() {
                        return None;
                    }
                    window = window.saturating_mul(2);
                    continue;
                }
            };

            // A delimiter starting at or before `found.1` could still have been cut off by the
            // window, which would make the other delimiter the closer one. Grow the window until
            // that is ruled out.
            let needed = found.1 + inline_len.max(block_len);
            if needed > end && end < input.len() {
                window = needed;
                continue;
            }
            return Some(found);
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
    use std::fmt::Write;

    /// Scan the input and reassemble it, marking up the snippets instead of converting them.
    fn replace(
        input: &'static str,
        inline_delim: (&str, &str),
        block_delim: (&str, &str),
        ignore_escaped_delim: bool,
    ) -> Result<String, ConversionError<'static>> {
        let replacer = Replacer::new(inline_delim, block_delim, ignore_escaped_delim);
        let scan = replacer.scan(input)?;

        let mut result = String::new();
        for ((latex, display), site) in scan.snippets.iter().zip(&scan.sites) {
            result.push_str(site.preceding_text);
            match display {
                MathDisplay::Inline => write!(result, "[T1:{latex}]").unwrap(),
                MathDisplay::Block => write!(result, "[T2:{latex}]").unwrap(),
            }
        }
        result.push_str(scan.trailing_text);
        Ok(result)
    }

    #[test]
    fn test_basic_replacement() {
        let input = "Hello $world$ and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, "Hello [T1:world] and [T2:universe]");
    }

    /// The search window in `find_next_delimiter` starts at 1024 bytes, so anything beyond that
    /// is only found after the window has grown.
    #[test]
    fn test_delimiters_beyond_search_window() {
        let filler = "a".repeat(5000);
        let input: &'static str = format!("{filler}$world$ and $$universe$${filler}").leak();
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(
            result,
            format!("{filler}[T1:world] and [T2:universe]{filler}")
        );
    }

    /// Only one of the two delimiter types occurs, so the other one is never found no matter how
    /// far the window grows.
    #[test]
    fn test_one_delimiter_type_absent_beyond_window() {
        let filler = "a".repeat(5000);
        let input: &'static str = format!("{filler}\\(world\\){filler}").leak();
        let result = replace(input, ("\\(", "\\)"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, format!("{filler}[T1:world]{filler}"));
    }

    /// A block delimiter that starts one byte before the window ends is initially invisible (the
    /// window cuts it in half) while the inline delimiter sharing its first byte is found right
    /// away. The window has to grow before the tie can be resolved in favor of block display.
    #[test]
    fn test_delimiter_straddling_window_end() {
        let filler = "a".repeat(1023);
        let input: &'static str = format!("{filler}$$universe$$").leak();
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap();
        assert_eq!(result, format!("{filler}[T2:universe]"));
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
        std::assert_matches!(
            result,
            ConversionError(7, ConvErrKind::MismatchedDelimiters(15), _)
        );
    }

    #[test]
    fn test_nested_delimiters2() {
        let input = "Nested $outer $$inner$$ delimiter$";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        std::assert_matches!(
            result,
            ConversionError(7, ConvErrKind::MismatchedDelimiters(14), _)
        );
    }

    #[test]
    fn test_mismatched_unclosed() {
        let input = "Unclosed $delimiter";
        let result = replace(input, ("$", "$"), ("$$", "$$"), false).unwrap_err();
        println!("{}", result);
        std::assert_matches!(
            result,
            ConversionError(9, ConvErrKind::UnclosedDelimiter, _)
        );
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
        std::assert_matches!(
            result,
            ConversionError(9, ConvErrKind::MismatchedDelimiters(16), _)
        );
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
        std::assert_matches!(
            result,
            ConversionError(4, ConvErrKind::MismatchedDelimiters(19), _)
        );
    }

    #[test]
    fn test_asymmetric_delimiters_nested2() {
        let input = r"let \(a=1 and \[b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        std::assert_matches!(
            result,
            ConversionError(14, ConvErrKind::NestedDelimiters, _)
        );
    }

    #[test]
    fn test_asymmetric_delimiters_nested3() {
        let input = r"let \(a=1 and \(b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        std::assert_matches!(
            result,
            ConversionError(14, ConvErrKind::NestedDelimiters, _)
        );
    }

    #[test]
    fn test_asymmetric_delimiters_unclosed() {
        let input = r"let \(a=1 and b=2.";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"), false).unwrap_err();
        println!("{}", result);
        std::assert_matches!(
            result,
            ConversionError(4, ConvErrKind::UnclosedDelimiter, _)
        );
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
}
