use const_fnv1a_hash::fnv1a_hash_64;
use std::fmt::Display;

/// A unique identifier for a type or value used in the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

impl Id {
    /// Creates a new protocol ID with the given number.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Creates a pseudorandom ID by hashing the string identifier.
    pub const fn from_string(string: &'static str) -> Self {
        Self(fnv1a_hash_64(string.as_bytes(), None))
    }

    /// Gets the underlying ID number.
    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl From<u64> for Id {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

impl From<Id> for u64 {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
