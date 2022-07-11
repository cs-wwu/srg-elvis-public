use crate::core::control::Primitive;
use std::fmt::{self, Display};
use thiserror::Error as ThisError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    pub const CURRENT_NETWORK: Self = Self([0u8, 0, 0, 0]);
    pub const PRIVATE_NETWORK: Self = Self([10u8, 0, 0, 0]);
    pub const LOCALHOST: Self = Self([127u8, 0, 0, 1]);
    pub const SUBNET: Self = Self([255u8, 255, 255, 255]);

    pub fn new(address: impl Into<Self>) -> Self {
        address.into()
    }

    pub fn to_u32(self) -> u32 {
        self.into()
    }

    pub fn to_bytes(self) -> [u8; 4] {
        self.into()
    }
}

impl Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = <[u8; 4]>::from(*self);
        write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

impl From<u32> for Ipv4Address {
    fn from(n: u32) -> Self {
        Self::from(n.to_be_bytes())
    }
}

impl From<[u8; 4]> for Ipv4Address {
    fn from(n: [u8; 4]) -> Self {
        Self(n)
    }
}

impl From<Ipv4Address> for u32 {
    fn from(address: Ipv4Address) -> Self {
        u32::from_be_bytes(address.0)
    }
}

impl From<Ipv4Address> for [u8; 4] {
    fn from(address: Ipv4Address) -> Self {
        address.0
    }
}

impl TryFrom<Primitive> for Ipv4Address {
    type Error = Ipv4AddressError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        Ok(value
            .to_u32()
            .ok_or(Ipv4AddressError::WrongPrimitive)?
            .into())
    }
}

impl From<Ipv4Address> for Primitive {
    fn from(address: Ipv4Address) -> Self {
        Primitive::U32(address.into())
    }
}

#[derive(Debug, ThisError)]
pub enum Ipv4AddressError {
    #[error("The primitive is the wrong type")]
    WrongPrimitive,
}
