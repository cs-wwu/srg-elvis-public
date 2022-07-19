use super::{Control, Primitive};
use std::{
    fmt::{self, Display},
    hash::Hash,
};
use thiserror::Error as ThisError;

/// Provides an interface for setting particular keys on a [`Control`].
///
/// In particular, protocols can expose monomorphized control values that act as
/// getters and setters for particular properties on a `Control` such that the
/// correct key and type are correctly adhered to by users. For example, a UDP
/// implementation would provide control values for local and remote port
/// numbers so that upstream and downstream protocols can exchange that
/// information in a consistent way. A control value wraps a type that can be
/// converted to and from a [`Primitive`] and provides functions to get and set
/// that value on a `Control`. The first generic parameter is the value type to
/// wrap. The second generic parameter is a `const` generic that specifies the
/// key to use on that control.
#[derive(Debug, Clone, Copy)]
pub struct ControlValue<const K: u64, V>(V);

impl<const K: u64, V> TryFrom<&Control> for ControlValue<K, V>
where
    V: TryFrom<Primitive>,
{
    type Error = ControlValueError<<V as TryFrom<Primitive>>::Error>;

    fn try_from(control: &Control) -> Result<Self, Self::Error> {
        Ok(Self(V::try_from(
            control.get(K).ok_or(ControlValueError::Missing(K))?,
        )?))
    }
}

impl<const K: u64, V> ControlValue<K, V>
where
    V: Into<Primitive>,
{
    /// Set the given `value` on the `control`
    pub fn set(control: &mut Control, value: V) {
        control.insert(K, value)
    }

    /// Set the wrapped value on the `control`
    pub fn apply(self, control: &mut Control) {
        control.insert(K, self.0)
    }
}

impl<const K: u64, V> ControlValue<K, V> {
    /// Create a new control value to wrap the `value`.
    pub fn new(value: V) -> Self {
        Self(value)
    }

    /// Retrieve the wrapped value.
    pub fn into_inner(self) -> V {
        self.0
    }
}

impl<const K: u64, V> ControlValue<K, V>
where
    V: TryFrom<Primitive>,
    <V as TryFrom<Primitive>>::Error: std::fmt::Debug,
{
    pub fn get(control: &Control) -> V {
        Self::try_from(control).unwrap().into_inner()
    }
}

impl<const K: u64, V> PartialEq for ControlValue<K, V>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<const K: u64, V> Eq for ControlValue<K, V> where V: PartialEq {}

impl<const K: u64, V> Hash for ControlValue<K, V>
where
    V: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<const K: u64, V> Display for ControlValue<K, V>
where
    V: Display,
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

macro_rules! make_key {
    ($key:ident) => {
        struct $key;

        impl $key {
            pub const KEY: u64 = std::intrinsics::type_id::<Self>();
        }
    };
}

pub(crate) use {from_impls, make_key};

/// An error occuring as a result of some operation on a [`ControlValue`].
#[derive(Debug, ThisError)]
pub enum ControlValueError<E> {
    #[error("Missing control key")]
    Missing(u64),
    #[error("{0}")]
    Invalid(#[from] E),
}
