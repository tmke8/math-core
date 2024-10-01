use std::fmt;

use memchr::memchr2;

#[derive(PartialEq, Clone, Copy, Debug)]
enum DelimiterType {
    Inline = 1,
    Display,
}

#[derive(Debug, PartialEq)]
pub enum ReplaceError {
    UnclosedDelimiter,
    NestedDelimiters,
    MismatchedDelimiters,
}
impl fmt::Display for ReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReplaceError::UnclosedDelimiter => write!(f, "Unclosed delimiter"),
            ReplaceError::NestedDelimiters => write!(f, "Nested delimiters are not allowed"),
            ReplaceError::MismatchedDelimiters => write!(f, "Unmatched delimiters"),
        }
    }
}
impl std::error::Error for ReplaceError {}

/// Replaces the content of inline and display math delimiters in a LaTeX string.
///
/// Any kind of nesting of delimiters is not allowed.
pub fn replace_slow_and_correct(
    input: &str,
    inline_delim: (&str, &str),
    display_delim: (&str, &str),
) -> Result<String, ReplaceError> {
    let mut result = String::new();
    let mut current_pos = 0;

    while current_pos < input.len() {
        let remaining = &input[current_pos..];

        // Find the next occurrence of any opening delimiter
        let (opening_type, start, opening_delim_len) = match (
            remaining
                .find(inline_delim.0)
                .map(|i| (DelimiterType::Inline, i, inline_delim.0)),
            remaining
                .find(display_delim.0)
                .map(|i| (DelimiterType::Display, i, display_delim.0)),
        ) {
            (Some((t1, i1, d1)), Some((t2, i2, d2))) => {
                // The inline delimiter could be a substring of the display delimiter
                // (e.g., `$` for inline vs `$$` for display)
                // so, if we have i2 == i1, we treat this as a display delimiter.
                if i2 <= i1 {
                    (t2, i2, d2.len())
                } else {
                    (t1, i1, d1.len())
                }
            }
            (Some((t, i, d)), None) | (None, Some((t, i, d))) => (t, i, d.len()),
            (None, None) => {
                // No more opening delimiters found
                result.push_str(&remaining);
                break;
            }
        };

        let start = current_pos + start;
        // Append everything before the opening delimiter
        result.push_str(&input[current_pos..start]);
        let start = start + opening_delim_len;
        let remaining = &input[start..];

        // Find the next occurrence of any closing delimiter
        let (closing_type, end, closing_delim_len) = match (
            remaining
                .find(inline_delim.1)
                .map(|i| (DelimiterType::Inline, i, inline_delim.1)),
            remaining
                .find(display_delim.1)
                .map(|i| (DelimiterType::Display, i, display_delim.1)),
        ) {
            (Some((t1, i1, d1)), Some((t2, i2, d2))) => {
                // The inline delimiter could be a substring of the display delimiter
                // (e.g., `$` for inline vs `$$` for display)
                // so, if we have i2 == i1, we treat this as a display delimiter.
                if i2 <= i1 {
                    (t2, i2, d2.len())
                } else {
                    (t1, i1, d1.len())
                }
            }
            (Some((t, i, d)), None) | (None, Some((t, i, d))) => (t, i, d.len()),
            (None, None) => {
                // No closing delimiter found
                return Err(ReplaceError::UnclosedDelimiter);
            }
        };

        if opening_type != closing_type {
            // Mismatched delimiters
            return Err(ReplaceError::MismatchedDelimiters);
        }

        let end = start + end;
        // Get the content between delimiters
        let content = &input[start..end];
        // Check whether any *opening* delimiters are present in the content
        if content.contains(inline_delim.0) || content.contains(display_delim.0) {
            // Nested delimiters
            return Err(ReplaceError::NestedDelimiters);
        }
        // Convert the content
        let converted = convert(content, opening_type);
        result.push_str(&converted);
        // Update current position
        current_pos = end + closing_delim_len;
    }

    Ok(result)
}

pub fn replace(
    input: &str,
    inline_delim: (&str, &str),
    display_delim: (&str, &str),
) -> Result<String, ReplaceError> {
    let mut result = String::new();
    let mut current_pos = 0;

    while current_pos < input.len() {
        let remaining = &input[current_pos..];

        // Find the next occurrence of any opening delimiter
        let opening = find_next_delimiter(remaining, inline_delim.0, display_delim.0);

        let Some((open_typ, idx, opening_delim_len)) = opening else {
            // No more opening delimiters found
            result.push_str(&remaining);
            break;
        };

        let start = current_pos + idx;
        // Append everything before the opening delimiter
        result.push_str(&input[current_pos..start]);
        let start = start + opening_delim_len;
        let remaining = &input[start..];
        println!("remaining: \"{}\"", remaining);
        println!("start: {}", start);
        println!("open_typ: {:?}", open_typ);

        // Find the next occurrence of any closing delimiter
        let closing = find_next_delimiter(remaining, inline_delim.1, display_delim.1);

        let Some((close_typ, idx, closing_delim_len)) = closing else {
            // No closing delimiter found
            return Err(ReplaceError::UnclosedDelimiter);
        };

        if open_typ != close_typ {
            return Err(ReplaceError::MismatchedDelimiters);
        }

        let end = start + idx;
        // Get the content between delimiters
        let content = &input[start..end];
        // Check whether any *opening* delimiters are present in the content
        if find_next_delimiter(content, inline_delim.0, display_delim.0).is_some() {
            return Err(ReplaceError::NestedDelimiters);
        }
        // Convert the content
        let converted = convert(content, open_typ);
        result.push_str(&converted);
        // Update current position
        current_pos = end + closing_delim_len;
    }

    Ok(result)
}

fn find_next_delimiter(
    input: &str,
    delim1: &str,
    delim2: &str,
) -> Option<(DelimiterType, usize, usize)> {
    let delim1_first_byte = delim1.as_bytes()[0];
    let delim2_first_byte = delim2.as_bytes()[0];
    let mut current_pos = 0;

    while let Some(offset) = memchr2(
        delim1_first_byte,
        delim2_first_byte,
        input[current_pos..].as_bytes(),
    ) {
        let idx = current_pos + offset;
        if input[idx..].starts_with(delim2) {
            return Some((DelimiterType::Display, idx, delim2.len()));
        } else if input[idx..].starts_with(delim1) {
            return Some((DelimiterType::Inline, idx, delim1.len()));
        } else {
            current_pos = idx + 1;
        }
    }
    None
}

// Mock convert function for testing
fn convert(content: &str, typ: DelimiterType) -> String {
    match typ {
        DelimiterType::Inline => format!("[T1:{}]", content),
        DelimiterType::Display => format!("[T2:{}]", content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_replacement() {
        let input = "Hello $world$ and $$universe$$";
        let result = replace(input, ("$", "$"), ("$$", "$$")).unwrap();
        assert_eq!(result, "Hello [T1:world] and [T2:universe]");
    }

    #[test]
    fn test_nested_delimiters() {
        let input = "Nested $$outer $inner$ delimiter$$";
        let result = replace(input, ("$", "$"), ("$$", "$$"));
        assert_eq!(result.unwrap_err(), ReplaceError::MismatchedDelimiters);
    }

    #[test]
    fn test_nested_delimiters2() {
        let input = "Nested $outer $$inner$$ delimiter$";
        let result = replace(input, ("$", "$"), ("$$", "$$"));
        assert_eq!(result.unwrap_err(), ReplaceError::MismatchedDelimiters);
    }

    #[test]
    fn test_mismatched_unclosed() {
        let input = "Unclosed $delimiter";
        let result = replace(input, ("$", "$"), ("$$", "$$"));
        assert_eq!(result.unwrap_err(), ReplaceError::UnclosedDelimiter);
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let result = replace(input, ("$", "$"), ("$$", "$$")).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_delimiters() {
        let input = "Hello, world!";
        let result = replace(input, ("$", "$"), ("$$", "$$")).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_multiple_replacements() {
        let input = "$a$ then $$b$$ then $c$ and $$d$$";
        let result = replace(input, ("$", "$"), ("$$", "$$")).unwrap();
        assert_eq!(result, "[T1:a] then [T2:b] then [T1:c] and [T2:d]");
    }

    #[test]
    fn test_mismatched_delimiters() {
        let input = "Mismatch $$ and $ signs";
        let result = replace(input, ("$", "$"), ("$$", "$$"));
        assert_eq!(result.unwrap_err(), ReplaceError::MismatchedDelimiters);
    }

    #[test]
    fn test_identical_delimiters() {
        let input = "|a| and ||b||";
        let result = replace(input, ("|", "|"), ("||", "||")).unwrap();
        assert_eq!(result, "[T1:a] and [T2:b]");
    }

    #[test]
    fn test_asymmetric_delimiters() {
        let input = r"let \(a=1\) and \[b=2\].";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]")).unwrap();
        assert_eq!(result, "let [T1:a=1] and [T2:b=2].");
    }

    #[test]
    fn test_asymmetric_delimiters_nested() {
        let input = r"let \(a=1 and \[b=2\]\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"));
        assert_eq!(result.unwrap_err(), ReplaceError::MismatchedDelimiters);
    }

    #[test]
    fn test_asymmetric_delimiters_nested2() {
        let input = r"let \(a=1 and \[b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"));
        assert_eq!(result.unwrap_err(), ReplaceError::NestedDelimiters);
    }

    #[test]
    fn test_asymmetric_delimiters_nested3() {
        let input = r"let \(a=1 and \(b=2\).";
        let result = replace(input, (r"\(", r"\)"), (r"\[", r"\]"));
        assert_eq!(result.unwrap_err(), ReplaceError::NestedDelimiters);
    }
}
