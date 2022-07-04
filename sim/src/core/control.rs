use std::collections::{hash_map::Entry, HashMap};
use thiserror::Error as ThisError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control(HashMap<ControlKey, Primitive>);

impl Control {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with(self, key: ControlKey, value: impl Into<Primitive>) -> Self {
        self.with_inner(key, value.into())
    }

    fn with_inner(mut self, key: ControlKey, value: Primitive) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert(&mut self, key: ControlKey, value: impl Into<Primitive>) -> Option<Primitive> {
        self.insert_inner(key, value.into())
    }

    fn insert_inner(&mut self, key: ControlKey, value: Primitive) -> Option<Primitive> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &ControlKey) -> Option<Primitive> {
        self.0.get(key).cloned()
    }

    pub fn entry(&mut self, key: ControlKey) -> Entry<ControlKey, Primitive> {
        self.0.entry(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKey {
    LocalAddress,
    RemoteAddress,
    SourcePort,
    DestinationPort,
    NetworkIndex,
    ProtocolId,
    Other(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
}

impl Primitive {
    pub fn to_u8(self) -> Result<u8, PrimitiveError> {
        match self {
            Self::U8(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_u16(self) -> Result<u16, PrimitiveError> {
        match self {
            Self::U16(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_u32(self) -> Result<u32, PrimitiveError> {
        match self {
            Self::U32(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_u64(self) -> Result<u64, PrimitiveError> {
        match self {
            Self::U64(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }
    pub fn to_u128(self) -> Result<u128, PrimitiveError> {
        match self {
            Self::U128(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_i8(self) -> Result<i8, PrimitiveError> {
        match self {
            Self::I8(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_i16(self) -> Result<i16, PrimitiveError> {
        match self {
            Self::I16(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_i32(self) -> Result<i32, PrimitiveError> {
        match self {
            Self::I32(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }

    pub fn to_i64(self) -> Result<i64, PrimitiveError> {
        match self {
            Self::I64(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }
    pub fn to_i128(self) -> Result<i128, PrimitiveError> {
        match self {
            Self::I128(value) => Ok(value),
            _ => Err(PrimitiveError::WrongPrimitiveKind),
        }
    }
}

#[derive(Debug, ThisError)]
pub enum PrimitiveError {
    #[error("Tried to unwrap into the wrong primitive type")]
    WrongPrimitiveKind,
}

impl From<u8> for Primitive {
    fn from(value: u8) -> Self {
        Self::U8(value)
    }
}

impl From<u16> for Primitive {
    fn from(value: u16) -> Self {
        Self::U16(value)
    }
}

impl From<u32> for Primitive {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

impl From<u64> for Primitive {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl From<u128> for Primitive {
    fn from(value: u128) -> Self {
        Self::U128(value)
    }
}

impl From<i8> for Primitive {
    fn from(value: i8) -> Self {
        Self::I8(value)
    }
}

impl From<i16> for Primitive {
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}

impl From<i32> for Primitive {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<i64> for Primitive {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<i128> for Primitive {
    fn from(value: i128) -> Self {
        Self::I128(value)
    }
}
