pub(crate) fn split_on_ascii(s: &str, c: u8) -> impl Iterator<Item = &str> {
    s.as_bytes()
        .split(move |b| c.is_ascii() && *b == c)
        // SAFETY: we checked, above, that `c` is ASCII,
        // and splitting UTF-8 on an ASCII byte results in valid UTF-8 substrings
        .map(|bytes| unsafe { str::from_utf8_unchecked(bytes) })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_on_comma() {
        assert_eq!(
            split_on_ascii("1,2,3", b',').collect::<Vec<&str>>(),
            vec!["1", "2", "3"]
        );
        assert_eq!(
            split_on_ascii("1a,2a,3a", b',').collect::<Vec<&str>>(),
            vec!["1a", "2a", "3a"]
        );
        assert_eq!(
            split_on_ascii(",,", b',').collect::<Vec<&str>>(),
            vec!["", "", ""]
        );
    }
    #[test]
    fn split_on_emoji_first_byte() {
        let emoji = "😁";
        let c = emoji.as_bytes()[0];
        assert_eq!(
            split_on_ascii("1😁2😁3", c).collect::<Vec<&str>>(),
            vec!["1😁2😁3"]
        );
        assert_eq!(
            split_on_ascii("1a😁2a😁3a", c).collect::<Vec<&str>>(),
            vec!["1a😁2a😁3a"]
        );
        assert_eq!(
            split_on_ascii("😁😁", c).collect::<Vec<&str>>(),
            vec!["😁😁"]
        );
    }
}
