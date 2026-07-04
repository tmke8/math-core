use core::fmt;

use percent_encoding::{AsciiSet, CONTROLS};
#[cfg(feature = "serde")]
use serde::Serialize;

/// A wrapper around `str` to do HTML escaping
/// in the `Display` impl.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct EscapeHtml<'a>(pub &'a str);

impl fmt::Display for EscapeHtml<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '<' => write!(f, "&lt;")?,
                '>' => write!(f, "&gt;")?,
                '&' => write!(f, "&amp;")?,
                '"' => write!(f, "&quot;")?,
                '\'' => write!(f, "&#39;")?,
                _ => write!(f, "{c}")?,
            }
        }
        Ok(())
    }
}

/// According to: https://stackoverflow.com/a/26119120
pub const FRAGMENT_SAFE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');
