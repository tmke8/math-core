use crate::attribute::TextTransform;
use crate::error::GetUnwrap;

/// This helper type is there to make string slices at least a little bit safe.
#[derive(Debug)]
#[repr(transparent)]
struct StrBound(usize);

#[derive(Debug)]
pub struct StrReference(StrBound, StrBound);

impl StrReference {
    #[inline]
    pub fn as_str<'buffer>(&self, buffer: &'buffer Buffer) -> &'buffer str {
        buffer.buffer.get_unwrap(self.0 .0..self.1 .0)
    }
}

#[derive(Debug)]
pub struct Buffer {
    buffer: String,
}

impl Buffer {
    pub fn new(size_hint: usize) -> Self {
        Buffer {
            buffer: String::with_capacity(size_hint),
        }
    }

    #[inline]
    pub fn extend<I: Iterator<Item = char>>(&mut self, iter: I) -> StrReference {
        let start = self.end();
        self.buffer.extend(iter);
        let end = self.end();
        StrReference(start, end)
    }

    /// Copy the contents of the given reference to the end of the buffer.
    ///
    /// If the given reference is invalid, this function will panic.
    /// However, on WASM, this function will instead do nothing.
    fn extend_from_within(&mut self, reference: &StrReference) -> StrReference {
        let start = self.end();
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(self.buffer.is_char_boundary(reference.0 .0));
            assert!(self.buffer.is_char_boundary(reference.1 .0));
            assert!(reference.0 .0 <= reference.1 .0);
            assert!(reference.1 .0 <= self.buffer.len());
        }
        // SAFETY: the bounds have been checked above
        unsafe {
            let begin = reference.0 .0;
            let end = reference.1 .0;
            let as_vec = self.buffer.as_mut_vec();
            // The following conditions should always hold true, but we check them
            // so that the compiler knows that this cannot panic.
            if begin <= end && begin < as_vec.len() && end <= as_vec.len() {
                as_vec.extend_from_within(begin..end);
            }
        }
        let end = self.end();
        StrReference(start, end)
    }

    pub fn transform_and_push(&mut self, input: &str, tf: TextTransform) -> StrReference {
        self.extend(input.chars().map(|c| tf.transform(c)))
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        let start = self.end();
        self.buffer.push_str(string);
        let end = self.end();
        StrReference(start, end)
    }

    #[inline]
    fn end(&self) -> StrBound {
        StrBound(self.buffer.len())
    }

    pub fn get_builder(&mut self) -> StringBuilder {
        StringBuilder::new(self)
    }
}

/// A helper type to safely build a string in the buffer from multiple pieces.
///
/// This takes an exclusive reference to the buffer and keeps track of the start
/// of the string being built. This guarantees that upon finishing, the string
/// has valid bounds and nothing else was written to the buffer in the meantime.
pub struct StringBuilder<'buffer> {
    buffer: &'buffer mut Buffer,
    start: StrBound,
}

impl<'buffer> StringBuilder<'buffer> {
    pub fn new(buffer: &'buffer mut Buffer) -> Self {
        let start = buffer.end();
        StringBuilder { buffer, start }
    }

    pub fn extend_from_within(&mut self, reference: &StrReference) -> StrReference {
        self.buffer.extend_from_within(reference)
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        self.buffer.push_str(string)
    }

    pub fn push_char(&mut self, ch: char) {
        self.buffer.buffer.push(ch);
    }

    pub fn transform_and_push(&mut self, input: &str, tf: TextTransform) -> StrReference {
        self.buffer.transform_and_push(input, tf)
    }

    pub fn finish(self) -> StrReference {
        StrReference(self.start, self.buffer.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_extend() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.extend("Hello, world!".chars());
        assert_eq!(str_ref.as_str(&buffer), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.push_str("Hello, world!");
        assert_eq!(str_ref.as_str(&buffer), "Hello, world!");
    }

    #[test]
    fn buffer_manual_reference() {
        let mut buffer = Buffer::new(0);
        let mut builder = buffer.get_builder();
        assert_eq!(builder.start.0, 0);
        builder.push_char('H');
        builder.push_char('i');
        builder.push_char('↩'); // This is a multi-byte character.
        let str_ref = builder.finish();
        assert_eq!(str_ref.1 .0, 5);
        assert_eq!(str_ref.as_str(&buffer), "Hi↩");
    }
}
