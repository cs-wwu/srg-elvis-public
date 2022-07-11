use super::ipv4_address::Ipv4Address;
use crate::core::{control::Primitive, Control};
use thiserror::Error as ThisError;

static LOCAL_ADDRESS_KEY: &str = "ipv4_local_address";
static REMOTE_ADDRESS_KEY: &str = "ipv4_remote_address";

pub struct ControlValue<T: TryFrom<Primitive>, const K: &'static str>(T)
where
    T: TryFrom<Primitive> + Into<Primitive>;

impl<T, const K: &'static str> TryFrom<&Control> for ControlValue<T, K>
where
    T: TryFrom<Primitive> + Into<Primitive>,
{
    type Error = ControlValueError<<T as TryFrom<Primitive>>::Error>;

    fn try_from(control: &Control) -> Result<Self, Self::Error> {
        Ok(Self(T::try_from(
            control
                .get(LOCAL_ADDRESS_KEY)
                .ok_or(ControlValueError::Missing(K))?,
        )?))
    }
}

impl<T, const K: &'static str> ControlValue<T, K>
where
    T: TryFrom<Primitive> + Into<Primitive>,
{
    pub fn set(&self, control: &mut Control) {
        control.insert(K, self.0)
    }
}

impl<T, const K: &'static str> ControlValue<T, K>
where
    T: TryFrom<Primitive> + Into<Primitive>,
{
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, const K: &'static str> From<T> for ControlValue<T, K>
where
    T: TryFrom<Primitive> + Into<Primitive>,
{
    fn from(t: T) -> Self {
        Self(t)
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
