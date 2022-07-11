use std::{collections::HashMap, fmt, fmt::Display, hash::Hash};
use thiserror::Error as ThisError;

mod primitive;
pub use primitive::*;

#[derive(Debug, Clone, Copy)]
struct StaticStr(&'static str);

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

/// A key-value store with which to exchange data between protocols.
///
/// [`Protocol`](super::Protocol)s often need to pass information to one another
/// such as lists of participants, information extracted from headers, and
/// configuration for opening a session. A control facilitates passing such
/// information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control(HashMap<StaticStr, Primitive>);

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
        self.0.insert(key.into(), value);
    }

    /// Gets the value for the given key.
    pub fn get(&self, key: &'static str) -> Option<Primitive> {
        self.0.get(&key.into()).cloned()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ControlValue<T, const K: &'static str>(T);

impl<T, const K: &'static str> TryFrom<&Control> for ControlValue<T, K>
where
    T: TryFrom<Primitive>,
{
    type Error = ControlValueError<<T as TryFrom<Primitive>>::Error>;

    fn try_from(control: &Control) -> Result<Self, Self::Error> {
        Ok(Self(T::try_from(
            control.get(K).ok_or(ControlValueError::Missing(K))?,
        )?))
    }
}

impl<T, const K: &'static str> ControlValue<T, K>
where
    T: Into<Primitive>,
{
    pub fn set(control: &mut Control, value: T) {
        control.insert(K, value)
    }

    pub fn apply(self, control: &mut Control) {
        control.insert(K, self.0)
    }
}

impl<T, const K: &'static str> ControlValue<T, K> {
    pub fn new(t: T) -> Self {
        Self(t)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, const K: &'static str> ControlValue<T, K>
where
    T: TryFrom<Primitive>,
    <T as TryFrom<Primitive>>::Error: std::fmt::Debug,
{
    pub fn get(control: &Control) -> T {
        Self::try_from(control).unwrap().into_inner()
    }
}

impl<T, const K: &'static str> PartialEq for ControlValue<T, K>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, const K: &'static str> Eq for ControlValue<T, K> where T: PartialEq {}

impl<T, const K: &'static str> Hash for ControlValue<T, K>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T, const K: &'static str> Display for ControlValue<T, K>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! from_impls {
    ($control_value:ty, $t:ty) => {
        impl From<$t> for $control_value {
            fn from(t: $t) -> Self {
                Self::new(t.into())
            }
        }

        impl From<$control_value> for $t {
            fn from(value: $control_value) -> Self {
                value.into_inner().into()
            }
        }
    };
}

pub(crate) use from_impls;

#[derive(Debug, ThisError)]
pub enum ControlValueError<E> {
    #[error("Missing control key")]
    Missing(&'static str),
    #[error("{0}")]
    Invalid(#[from] E),
}
