use std::fmt::{self, Display};
use thiserror::Error as ThisError;

/// A value of some numeric primitive type.
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
    /// Get the contained `u8`.
    pub fn to_u8(self) -> Option<u8> {
        match self {
            Self::U8(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `u16`.
    pub fn to_u16(self) -> Option<u16> {
        match self {
            Self::U16(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `u32`.
    pub fn to_u32(self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `u64`.
    pub fn to_u64(self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `u128`.
    pub fn to_u128(self) -> Option<u128> {
        match self {
            Self::U128(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `i8`.
    pub fn to_i8(self) -> Option<i8> {
        match self {
            Self::I8(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `i16`.
    pub fn to_i16(self) -> Option<i16> {
        match self {
            Self::I16(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `i32`.
    pub fn to_i32(self) -> Option<i32> {
        match self {
            Self::I32(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `i64`.
    pub fn to_i64(self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `i128`.
    pub fn to_i128(self) -> Option<i128> {
        match self {
            Self::I128(value) => Some(value),
            _ => None,
        }
    }

    /// Get the contained `u8`.
    pub fn ok_u8(self) -> Result<u8, PrimitiveError> {
        match self {
            Self::U8(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::U8,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `u16`.
    pub fn ok_u16(self) -> Result<u16, PrimitiveError> {
        match self {
            Self::U16(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::U16,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `u32`.
    pub fn ok_u32(self) -> Result<u32, PrimitiveError> {
        match self {
            Self::U32(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::U32,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `u64`.
    pub fn ok_u64(self) -> Result<u64, PrimitiveError> {
        match self {
            Self::U64(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::U64,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `u128`.
    pub fn ok_u128(self) -> Result<u128, PrimitiveError> {
        match self {
            Self::U128(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::U128,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `i8`.
    pub fn ok_i8(self) -> Result<i8, PrimitiveError> {
        match self {
            Self::I8(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::I8,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `i16`.
    pub fn ok_i16(self) -> Result<i16, PrimitiveError> {
        match self {
            Self::I16(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::I16,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `i32`.
    pub fn ok_i32(self) -> Result<i32, PrimitiveError> {
        match self {
            Self::I32(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::I32,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `i64`.
    pub fn ok_i64(self) -> Result<i64, PrimitiveError> {
        match self {
            Self::I64(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::I64,
                actual: other.into(),
            }),
        }
    }

    /// Get the contained `i128`.
    pub fn ok_i128(self) -> Result<i128, PrimitiveError> {
        match self {
            Self::I128(value) => Ok(value),
            other => Err(PrimitiveError::WrongKind {
                expected: PrimitiveKind::I128,
                actual: other.into(),
            }),
        }
    }
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

impl TryFrom<Primitive> for u8 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_u8()
    }
}

impl TryFrom<Primitive> for u16 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_u16()
    }
}

impl TryFrom<Primitive> for u32 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_u32()
    }
}

impl TryFrom<Primitive> for u64 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_u64()
    }
}

impl TryFrom<Primitive> for u128 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_u128()
    }
}

impl TryFrom<Primitive> for i8 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_i8()
    }
}

impl TryFrom<Primitive> for i16 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_i16()
    }
}

impl TryFrom<Primitive> for i32 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_i32()
    }
}

impl TryFrom<Primitive> for i64 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_i64()
    }
}

impl TryFrom<Primitive> for i128 {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        value.ok_i128()
    }
}

/// Represents a variant of [`Primitive`], minus the contained value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl From<Primitive> for PrimitiveKind {
    fn from(primitive: Primitive) -> Self {
        match primitive {
            Primitive::U8(_) => Self::U8,
            Primitive::U16(_) => Self::U16,
            Primitive::U32(_) => Self::U32,
            Primitive::U64(_) => Self::U64,
            Primitive::U128(_) => Self::U128,
            Primitive::I8(_) => Self::I8,
            Primitive::I16(_) => Self::I16,
            Primitive::I32(_) => Self::I32,
            Primitive::I64(_) => Self::I64,
            Primitive::I128(_) => Self::I128,
        }
    }
}

impl Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PrimitiveKind::*;
        let s = match self {
            U8 => "U8",
            U16 => "U16",
            U32 => "U32",
            U64 => "U64",
            U128 => "U128",
            I8 => "I8",
            I16 => "I16",
            I32 => "I32",
            I64 => "I64",
            I128 => "I128",
        };
        write!(f, "{}", s)
    }
}

/// An error caused by a [`Primitive`]
#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveError {
    #[error("Expected {expected} but got {actual}")]
    WrongKind {
        expected: PrimitiveKind,
        actual: PrimitiveKind,
    },
}
