use std::borrow::Cow;

use memchr::memchr;

static HTML_RESERVED_MAP: phf::Map<&'static [u8], u8> = phf::phf_map! {
    b"#34" => b'"',
    b"quot" => b'"',
    b"#38" => b'&',
    b"amp" => b'&',
    b"#39" => b'\'',
    b"apos" => b'\'',
    b"#60" => b'<',
    b"lt" => b'<',
    b"#62" => b'>',
    b"gt" => b'>',
};

/// Replace HTML entities in the input string with their corresponding characters.
///
/// If the input contains no entities at all, it is returned borrowed and unchanged.
pub fn replace_html_entities(input: &str) -> Cow<'_, str> {
    let bytes = input.as_bytes();

    let Some(first_ampersand) = memchr(b'&', bytes) else {
        // No `&` character found, return the original input.
        return Cow::Borrowed(input);
    };
    let mut buffer = String::with_capacity(input.len());

    let mut last_end = 0;
    let mut next_start = first_ampersand;

    loop {
        // Copy the part between the last `&` and the current `&`.
        buffer.push_str(&input[last_end..next_start]);

        let entity_start = next_start + 1;
        let Some(index) = bytes[entity_start..].iter().position(|&b| b == b';') else {
            // No `;` character found, exit the loop.
            last_end = next_start;
            break;
        };
        let end = entity_start + index;

        // We use `next_start + 1` to skip the `&` character.
        if let Some(replacement) = HTML_RESERVED_MAP.get(&bytes[entity_start..end]) {
            unsafe {
                buffer.push_str(std::str::from_utf8_unchecked(&[*replacement]));
            }
        } else {
            // No match, copy the original string.
            buffer.push_str(&input[next_start..=end]);
        }

        // We use `end + 1` to include the `;` character.
        last_end = end + 1;

        // Check for the next ampersand
        match memchr(b'&', &bytes[last_end..]) {
            Some(idx) => next_start = last_end + idx,
            None => break,
        }
    }

    // Push the remaining part of the input
    buffer.push_str(&input[last_end..]);
    Cow::Owned(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_html_entities() {
        assert_eq!(replace_html_entities("you &amp; I"), "you & I");
        assert_eq!(replace_html_entities("&lt;hello&gt;"), "<hello>");
        assert_eq!(replace_html_entities("no entities"), "no entities");
        assert_eq!(replace_html_entities("&#34;quoted&#34;"), "\"quoted\"");
        assert_eq!(replace_html_entities("&apos;single&apos;"), "'single'");
        assert_eq!(
            replace_html_entities("mix &amp; &#60;match&#62;"),
            "mix & <match>"
        );
        assert_eq!(replace_html_entities("incomplete &amp"), "incomplete &amp");
        assert_eq!(
            replace_html_entities("unknown &nbsp; entity"),
            "unknown &nbsp; entity"
        );
        assert_eq!(replace_html_entities("at end &"), "at end &");
        assert_eq!(replace_html_entities("you &&amp; I"), "you &&amp; I");
    }
}
