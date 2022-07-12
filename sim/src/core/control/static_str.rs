use std::hash::Hash;

/// A newtype for `&str` that hashes based on the string pointer. This is
/// compatible with strings defined as `static` but not those defined as
/// `const`. See this [`Stack Overflow post`] for more details.
///
/// [`Stack Overflow post`]:
///     https://stackoverflow.com/questions/72905318/why-are-string-constant-pointers-different-across-crates-in-rust
#[derive(Debug, Clone, Copy)]
pub(super) struct StaticStr(&'static str);

impl From<&'static str> for StaticStr {
    fn from(s: &'static str) -> Self {
        Self(s)
    }
}

impl Hash for StaticStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl PartialEq for StaticStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for StaticStr {}
