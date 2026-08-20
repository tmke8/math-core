use lean_string::LeanStr;

/// A pool of strings which have been copied out of the input.
///
/// This exists so that tokens can refer to pieces of the input without borrowing it.
///
/// Strings are not deduplicated; two calls to [`StringPool::intern`] with the same string
/// yield two different [`InternedStr`] values.
#[derive(Debug, Default)]
pub struct StringPool;

impl StringPool {
    /// Copy `s` into the pool and return an index which can retrieve it again.
    #[inline]
    pub fn intern(&mut self, s: &str) -> LeanStr {
        LeanStr::from(s)
    }

    /// Forget all interned strings, invalidating every [`InternedStr`] of this pool.
    pub fn clear(&mut self) {}

    /// Retrieve a string which was previously interned in *this* pool.
    #[inline]
    pub fn get<'a>(&self, index: &'a LeanStr) -> &'a str {
        index.as_str()
    }
}
