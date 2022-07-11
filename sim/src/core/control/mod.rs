use std::collections::HashMap;
use thiserror::Error as ThisError;

mod primitive;
pub use primitive::Primitive;

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

pub trait Bounds = TryFrom<Primitive> + Into<Primitive> + Copy + std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub struct ControlValue<T: Bounds, const K: &'static str>(T);

impl<T: Bounds, const K: &'static str> TryFrom<&Control> for ControlValue<T, K> {
    type Error = ControlValueError<<T as TryFrom<Primitive>>::Error>;

    fn try_from(control: &Control) -> Result<Self, Self::Error> {
        Ok(Self(T::try_from(
            control.get(K).ok_or(ControlValueError::Missing(K))?,
        )?))
    }
}

impl<T: Bounds, const K: &'static str> From<T> for ControlValue<T, K> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

impl<T: Bounds, const K: &'static str> ControlValue<T, K> {
    pub fn set(control: &mut Control, value: T) {
        control.insert(K, value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Bounds, const K: &'static str> ControlValue<T, K>
where
    <T as TryFrom<Primitive>>::Error: std::fmt::Debug,
{
    pub fn get(control: &Control) -> T {
        Self::try_from(control).unwrap().into_inner()
    }
}

#[derive(Debug, ThisError)]
pub enum ControlValueError<E> {
    #[error("Missing control key")]
    Missing(&'static str),
    #[error("{0}")]
    Invalid(#[from] E),
}
