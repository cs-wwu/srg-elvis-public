use std::collections::HashMap;

mod primitive;
pub use primitive::Primitive;

mod static_str;
use static_str::StaticStr;

/// A key-value store with which to exchange data between protocols.
///
/// [`Protocol`](super::Protocol)s often need to pass information to one another
/// such as lists of participants, information extracted from headers, and
/// configuration for opening a session. A control facilitates passing such
/// information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control(HashMap<&'static str, Primitive>);

impl Control {
    /// Creates a new control.
    pub fn new() -> Self {
        Default::default()
    }

    /// A builder function that adds the given key-value pair to the control.
    ///
    /// See [`insert`](Self::insert) for more details.
    pub fn with(mut self, key: &'static str, value: impl Into<Primitive>) -> Self {
        self.insert_inner(key, value.into());
        self
    }

    /// Adds the given key-value pair to the control.
    ///
    /// `value` can be any numeric primitive of universally-defined size, such
    /// as an `i16` or a `u64`. `usize` and `isize` are not allowed because
    /// their sizes are platform-dependent.
    pub fn insert(&mut self, key: &'static str, value: impl Into<Primitive>) {
        self.insert_inner(key, value.into())
    }

    fn insert_inner(&mut self, key: &'static str, value: Primitive) {
        self.0.insert(key, value);
    }

    /// Gets the value for the given key.
    pub fn get(&self, key: &'static str) -> Option<Primitive> {
        self.0.get(&key).cloned()
    }
}
