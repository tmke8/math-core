use std::fmt::{Display,Formatter};

#[repr(transparent)]
pub struct StrJoiner([&'static str]);

impl StrJoiner {
    pub const fn from_slice<'a>(slice: &'a [&'static str]) -> &'a Self {
        // SAFETY: `[&'static str]` and `Parts` have the same memory layout.
        unsafe { &*(slice as *const [&'static str] as *const Self) }
    }
}

impl Display for StrJoiner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for elem in &self.0 {
            write!(f, "{elem}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    #[test]
    fn test_empty_slice() {
        let empty = StrJoiner::from_slice(&[]);
        assert_eq!(empty.to_string(), "");
    }

    #[test]
    fn test_single_element() {
        let single = StrJoiner::from_slice(&["test"]);
        assert_eq!(single.to_string(), "test");
    }

    #[test]
    fn test_multiple_elements() {
        let multiple = StrJoiner::from_slice(&["hello ", "world", "!"]);
        assert_eq!(multiple.to_string(), "hello world!");
    }

    #[test]
    fn test_with_empty_strings() {
        let with_empty = StrJoiner::from_slice(&["", "middle", ""]);
        assert_eq!(with_empty.to_string(), "middle");
    }

    #[test]
    fn test_with_whitespace() {
        let with_spaces = StrJoiner::from_slice(&[" ", "spaced", " ", "content", " "]);
        assert_eq!(with_spaces.to_string(), " spaced content ");
    }

    #[test]
    fn test_with_special_characters() {
        let special = StrJoiner::from_slice(&["new\nline", "\ttab", "\\escape"]);
        assert_eq!(special.to_string(), "new\nline\ttab\\escape");
    }

    #[test]
    fn test_large_slice() {
        let elements = ["a"; 1000]; // Array with 1000 "a" strings
        let large = StrJoiner::from_slice(&elements);
        assert_eq!(large.to_string(), "a".repeat(1000));
    }

    #[test]
    fn test_static_lifetime() {
        // This test ensures the static lifetime requirement works as expected
        static STATIC_STRS: [&'static str; 2] = ["static", "strings"];
        let joiner = StrJoiner::from_slice(&STATIC_STRS);
        assert_eq!(joiner.to_string(), "staticstrings");
    }

    #[test]
    fn test_in_format() {
        let s = format!("text: {}", StrJoiner::from_slice(&["red", ",", "blue"]));
        assert_eq!(s.as_str(), "text: red,blue");
    }
}
