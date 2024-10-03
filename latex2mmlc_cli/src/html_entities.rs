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

pub fn replace_html_entities(buffer: &mut String, input: &str) {
    // Clear the buffer and reserve enough space for the new string.
    buffer.clear();
    if buffer.capacity() < input.len() {
        buffer.reserve(input.len() - buffer.capacity());
    }

    let bytes = input.as_bytes();

    let Some(first_ampersand) = memchr(b'&', bytes) else {
        // No `&` character found, return the original input.
        buffer.push_str(input);
        return;
    };

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
            buffer.push_str(unsafe { std::str::from_utf8_unchecked(&[*replacement]) });
        } else {
            // No match, copy the original string.
            buffer.push_str(&input[next_start..=end]);
        };

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_html_entities() {
        let b = &mut String::new();
        replace_html_entities(b, "you &amp; I");
        b.as_str();
        assert_eq!(b.as_str(), "you & I");
        replace_html_entities(b, "&lt;hello&gt;");
        assert_eq!(b.as_str(), "<hello>");
        replace_html_entities(b, "no entities");
        assert_eq!(b.as_str(), "no entities");
        replace_html_entities(b, "&#34;quoted&#34;");
        assert_eq!(b.as_str(), "\"quoted\"");
        replace_html_entities(b, "&apos;single&apos;");
        assert_eq!(b.as_str(), "'single'");
        replace_html_entities(b, "mix &amp; &#60;match&#62;");
        assert_eq!(b.as_str(), "mix & <match>");
        replace_html_entities(b, "incomplete &amp");
        assert_eq!(b.as_str(), "incomplete &amp");
        replace_html_entities(b, "unknown &nbsp; entity");
        assert_eq!(b.as_str(), "unknown &nbsp; entity");
        replace_html_entities(b, "at end &");
        assert_eq!(b.as_str(), "at end &");
        replace_html_entities(b, "you &&amp; I");
        assert_eq!(b.as_str(), "you &&amp; I");
    }
}
