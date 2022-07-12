use super::{Control, Primitive};
use std::{
    fmt::{self, Display},
    hash::Hash,
};
use thiserror::Error as ThisError;

/// Simplifies the creation of interfaces for [`Control`].
///
/// A control value wraps a type that can be converted to and from a
/// [`Primitive`] and provides functions to get and set that value to a
/// `Control`. The first generic parameter is the value type to wrap. The second
/// generic parameter is a `const` generic that specifies the key to use on the
/// control.
///
/// # Examples
///
/// Creating a new control value type for setting a port number on a control:
///
/// ```
/// # use elvis::core::control::ControlValue;
/// pub type Port = ControlValue<u16, "my_port_key">;
/// ```
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
    /// Set the given `value` on the `control`
    pub fn set(control: &mut Control, value: T) {
        control.insert(K, value)
    }

    /// Set the wrapped value on the `control`
    pub fn apply(self, control: &mut Control) {
        control.insert(K, self.0)
    }
}

impl<T, const K: &'static str> ControlValue<T, K> {
    /// Create a new control value to wrap the `value`.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Retrieve the wrapped value.
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

/// Create bidirectional [`From`] implementations between the [`ControlValue`]
/// and some type.
///
/// Ideally, the type system would allow us to write a blanket implementation
/// such that a control value can be easily converted to and from any type the
/// wrapped type provides `From` implementations for. Unfortunately, this does
/// not seem to be possible at the moment. This macro simplifies the legwork of
/// writing these implementations for specific types. Unfortunately, given
/// Rust's coherence rules, this macro can only be used from inside the Elvis
/// crate. A redesign will probably be necessary at some point.
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

/// An error occuring as a result of some operation on a [`ControlValue`].
#[derive(Debug, ThisError)]
pub enum ControlValueError<E> {
    #[error("Missing control key")]
    Missing(&'static str),
    #[error("{0}")]
    Invalid(#[from] E),
}
