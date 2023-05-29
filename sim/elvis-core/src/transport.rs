use crate::protocols::{Tcp, Udp};
use std::{
    any::TypeId,
    error::Error,
    fmt::{self, Display, Formatter},
};

// TODO(hardint): This should probably be moved into the IPv4 parsing stuff

/// A byte used to indicate the protocol contained inside an IPv4 packet
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Transport {
    Tcp = 6,
    Udp = 17,
}

impl TryFrom<TypeId> for Transport {
    type Error = TransportConvertError;

    fn try_from(type_id: TypeId) -> Result<Self, Self::Error> {
        if type_id == TypeId::of::<Tcp>() {
            Ok(Self::Tcp)
        } else if type_id == TypeId::of::<Udp>() {
            Ok(Self::Udp)
        } else {
            Err(TransportConvertError)
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

impl TryFrom<u8> for Transport {
    type Error = TransportConvertError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            6 => Ok(Self::Tcp),
            17 => Ok(Self::Udp),
            _ => Err(TransportConvertError),
        }
    }
}

impl From<Transport> for u8 {
    fn from(value: Transport) -> Self {
        value as u8
    }
}

#[derive(Debug, Copy, Clone, Default, Hash)]
pub struct TransportConvertError;

impl Display for TransportConvertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "No protocol number for the given type ID")
    }
}

impl Error for TransportConvertError {}
