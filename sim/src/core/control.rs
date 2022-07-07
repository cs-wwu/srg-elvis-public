use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A key-value store for protocols to be able to exchange data, such as a list
/// of participants, information extracted from headers, or configuration for
/// opening a session.
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
    pub fn with(self, key: &'static str, value: impl Into<Primitive>) -> Self {
        self.with_inner(key, value.into())
    }

    fn with_inner(mut self, key: &'static str, value: Primitive) -> Self {
        self.insert_inner(key, value);
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

/// Since we only work with static strings for [`Control`], we use a newtype to
/// make the string hash based on its pointer to speed up map performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticStr(&'static str);

impl Hash for StaticStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.as_ptr() as usize)
    }
}

impl From<&'static str> for StaticStr {
    fn from(s: &'static str) -> Self {
        Self(s)
    }
}

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
