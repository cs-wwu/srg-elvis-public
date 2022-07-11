use super::ipv4_address::Ipv4Address;
use crate::core::{control::Primitive, Control};
use thiserror::Error as ThisError;

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

pub type LocalAddress = ControlValue<Ipv4Address, "ipv4_local_address">;
pub type RemoteAddress = ControlValue<Ipv4Address, "ipv4_remote_address">;

#[derive(Debug, ThisError)]
pub(super) enum Ipv4Error {
    #[error("Could not find a listen binding for the local address: {0}")]
    MissingListenBinding(Ipv4Address),
    #[error("Attempting to create a binding that already exists for source address {0}")]
    BindingExists(Ipv4Address),
    #[error("Attempting to create a session that already exists for {0} -> {1}")]
    SessionExists(Ipv4Address, Ipv4Address),
}
