/// Escapes special characters in `input` for safe inclusion in HTML content.
/// Specifically, it replaces:
/// - `&` with `&amp;`
/// - `<` with `&lt;`
/// - `>` with `&gt;`
///
/// This function uses `memchr` for efficient searching of special characters.
pub fn escape_html_content(output: &mut String, input: &str) {
    let output = unsafe { output.as_mut_vec() };
    let mut haystack = input.as_bytes();

    while let Some(index) = memchr::memchr3(b'&', b'<', b'>', haystack) {
        let Some((before, after)) = haystack.split_at_checked(index) else {
            break;
        };
        // Copy everything before the special character
        output.extend_from_slice(before);

        let Some((special_char, rest)) = after.split_first() else {
            break;
        };

        // Append the escaped version
        match special_char {
            b'&' => output.extend_from_slice(b"&amp;"),
            b'<' => output.extend_from_slice(b"&lt;"),
            b'>' => output.extend_from_slice(b"&gt;"),
            _ => {}
        }

        haystack = rest;
    }

    // Copy any remaining bytes after the last special character
    output.extend_from_slice(haystack);
}

/// Escapes special characters in `input` for safe inclusion in HTML attributes
/// which are enclosed in double quotes.
///
/// Specifically, it replaces:
/// - `&` with `&amp;`
/// - `"` with `&quot;`
///
/// In contrast to `escape_html_content`, this function does not use `memchr`
/// for optimization, as attributes are typically shorter strings.
pub fn escape_double_quoted_html_attribute(output: &mut String, input: &str) {
    let output = unsafe { output.as_mut_vec() };
    for ch in input.bytes() {
        match ch {
            b'&' => output.extend_from_slice(b"&amp;"),
            b'"' => output.extend_from_slice(b"&quot;"),
            _ => output.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut output = String::new();
        escape_html_content(&mut output, "");
        assert_eq!(output, "");
    }

    #[test]
    fn test_no_special_characters() {
        let mut output = String::new();
        escape_html_content(&mut output, "Hello, World!");
        assert_eq!(output, "Hello, World!");
    }

    #[test]
    fn test_escape_ampersand() {
        let mut output = String::new();
        escape_html_content(&mut output, "Tom & Jerry");
        assert_eq!(output, "Tom &amp; Jerry");
    }

    #[test]
    fn test_escape_less_than() {
        let mut output = String::new();
        escape_html_content(&mut output, "5 < 10");
        assert_eq!(output, "5 &lt; 10");
    }

    #[test]
    fn test_escape_greater_than() {
        let mut output = String::new();
        escape_html_content(&mut output, "10 > 5");
        assert_eq!(output, "10 &gt; 5");
    }

    #[test]
    fn test_all_special_characters() {
        let mut output = String::new();
        escape_html_content(&mut output, "<tag>&content</tag>");
        assert_eq!(output, "&lt;tag&gt;&amp;content&lt;/tag&gt;");
    }

    #[test]
    fn test_consecutive_special_characters() {
        let mut output = String::new();
        escape_html_content(&mut output, "&<>");
        assert_eq!(output, "&amp;&lt;&gt;");
    }

    #[test]
    fn test_special_at_start() {
        let mut output = String::new();
        escape_html_content(&mut output, "<html>");
        assert_eq!(output, "&lt;html&gt;");
    }

    #[test]
    fn test_special_at_end() {
        let mut output = String::new();
        escape_html_content(&mut output, "test&");
        assert_eq!(output, "test&amp;");
    }

    #[test]
    fn test_only_special_characters() {
        let mut output = String::new();
        escape_html_content(&mut output, "&&&<<<>>>");
        assert_eq!(output, "&amp;&amp;&amp;&lt;&lt;&lt;&gt;&gt;&gt;");
    }

    #[test]
    fn test_utf8_with_special_characters() {
        let mut output = String::new();
        escape_html_content(&mut output, "Hello 世界 & <test>");
        assert_eq!(output, "Hello 世界 &amp; &lt;test&gt;");
    }

    #[test]
    fn test_appends_to_existing_output() {
        let mut output = "prefix: ".to_string();
        escape_html_content(&mut output, "<tag>");
        assert_eq!(output, "prefix: &lt;tag&gt;");
    }

    #[test]
    fn test_long_text_with_few_escapes() {
        let mut output = String::new();
        let input = "This is a very long string with only one special character at the end: <";
        escape_html_content(&mut output, input);
        assert_eq!(
            output,
            "This is a very long string with only one special character at the end: &lt;"
        );
    }

    #[test]
    fn test_alternating_pattern() {
        let mut output = String::new();
        escape_html_content(&mut output, "a<b>c&d<e>f&");
        assert_eq!(output, "a&lt;b&gt;c&amp;d&lt;e&gt;f&amp;");
    }

    #[test]
    fn test_single_byte_inputs() {
        let mut output = String::new();
        escape_html_content(&mut output, "&");
        assert_eq!(output, "&amp;");

        output.clear();
        escape_html_content(&mut output, "<");
        assert_eq!(output, "&lt;");

        output.clear();
        escape_html_content(&mut output, ">");
        assert_eq!(output, "&gt;");

        output.clear();
        escape_html_content(&mut output, "a");
        assert_eq!(output, "a");
    }
}
