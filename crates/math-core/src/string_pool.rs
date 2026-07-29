use alloc::string::String;

use crate::error::GetUnwrap;

/// An index into a [`StringPool`].
///
/// This is only meaningful for the pool that produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InternedStr(usize, usize);

/// A pool of strings which have been copied out of the input.
///
/// This exists so that tokens can refer to pieces of the input without borrowing it.
///
/// Strings are not deduplicated; two calls to [`StringPool::intern`] with the same string
/// yield two different [`InternedStr`] values.
#[derive(Debug, Default)]
pub struct StringPool(String);

impl StringPool {
    /// Copy `s` into the pool and return an index which can retrieve it again.
    pub fn intern(&mut self, s: &str) -> InternedStr {
        let start = self.0.len();
        self.0.push_str(s);
        InternedStr(start, self.0.len())
    }

    /// Retrieve a string which was previously interned in *this* pool.
    pub fn get(&self, index: InternedStr) -> &str {
        // SAFETY: `InternedStr` can only be obtained from `intern()`, which returns
        // indices at `str` boundaries.
        self.0.get_unwrap(index.0..index.1)
    }
}
