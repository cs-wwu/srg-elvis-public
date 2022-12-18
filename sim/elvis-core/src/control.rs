//! Types for exchanging data between protocols.
//!
//! This module primarily implements the [`Control`] key-value store.

use self::primitive::PrimitiveError;
use crate::id::Id;
use std::collections::HashMap;
use thiserror::Error as ThisError;

pub(crate) mod primitive;
pub use primitive::Primitive;

/// A key for a [`Control`].
pub type Key = (Id, u64);

/// A key-value store with which to exchange data between protocols.
///
/// [`Protocol`](super::Protocol)s often need to pass information to one another
/// such as lists of participants, information extracted from headers, and
/// configuration for opening a session. A control facilitates passing such
/// information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control(HashMap<Key, Primitive>);

impl Control {
    /// Creates a new control.
    pub fn new() -> Self {
        Default::default()
    }

    /// A builder function that adds the given key-value pair to the control.
    ///
    /// See [`insert`](Self::insert) for more details.
    pub fn with(mut self, key: Key, value: impl Into<Primitive>) -> Self {
        self.insert_inner(key, value.into());
        self
    }

    /// Adds the given key-value pair to the control.
    ///
    /// `value` can be any numeric primitive of universally-defined size, such
    /// as an `i16` or a `u64`. `usize` and `isize` are not allowed because
    /// their sizes are platform-dependent.
    pub fn insert(&mut self, key: Key, value: impl Into<Primitive>) {
        self.insert_inner(key, value.into())
    }

    fn insert_inner(&mut self, key: Key, value: Primitive) {
        self.0.insert(key, value);
    }

    /// Gets the value for the given key.
    pub fn get(&self, key: Key) -> Result<Primitive, ControlError> {
        self.0.get(&key).cloned().ok_or(ControlError::Missing)
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    #[error("Control key missing")]
    Missing,
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
}
