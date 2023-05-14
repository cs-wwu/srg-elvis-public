use crate::protocols::{Tcp, Udp};
use std::{
    any::TypeId,
    error::Error,
    fmt::{self, Display, Formatter},
};

/// A byte used to indicate the protocol contained inside an IPv4 packet
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Transport {
    Tcp = 6,
    Udp = 17,
}

impl TryFrom<TypeId> for Transport {
    type Error = ProtocolNumberFromTypeIdError;

    fn try_from(type_id: TypeId) -> Result<Self, Self::Error> {
        if type_id == TypeId::of::<Tcp>() {
            Ok(Self::Tcp)
        } else if type_id == TypeId::of::<Udp>() {
            Ok(Self::Udp)
        } else {
            Err(ProtocolNumberFromTypeIdError)
        }
    }
}

impl From<Transport> for TypeId {
    fn from(protocol_number: Transport) -> Self {
        match protocol_number {
            Transport::Tcp => TypeId::of::<Tcp>(),
            Transport::Udp => TypeId::of::<Udp>(),
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Hash)]
pub struct ProtocolNumberFromTypeIdError;

impl Display for ProtocolNumberFromTypeIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "No protocol number for the given type ID")
    }
}

impl Error for ProtocolNumberFromTypeIdError {}
