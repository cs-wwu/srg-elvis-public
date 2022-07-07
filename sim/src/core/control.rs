use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Control(HashMap<&'static str, Primitive>);

impl Control {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with(self, key: &'static str, value: impl Into<Primitive>) -> Self {
        self.with_inner(key, value.into())
    }

    fn with_inner(mut self, key: &'static str, value: Primitive) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert(&mut self, key: &'static str, value: impl Into<Primitive>) {
        self.insert_inner(key, value.into())
    }

    fn insert_inner(&mut self, key: &'static str, value: Primitive) {
        self.0.insert(key, value);
    }

    pub fn get(&self, key: &'static str) -> Option<Primitive> {
        self.0.get(key).cloned()
    }
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
    pub fn to_u8(self) -> Option<u8> {
        match self {
            Self::U8(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_u16(self) -> Option<u16> {
        match self {
            Self::U16(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_u32(self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_u64(self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(value),
            _ => None,
        }
    }
    pub fn to_u128(self) -> Option<u128> {
        match self {
            Self::U128(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_i8(self) -> Option<i8> {
        match self {
            Self::I8(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_i16(self) -> Option<i16> {
        match self {
            Self::I16(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_i32(self) -> Option<i32> {
        match self {
            Self::I32(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_i64(self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(value),
            _ => None,
        }
    }
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
