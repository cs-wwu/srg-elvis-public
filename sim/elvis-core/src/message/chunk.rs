use std::sync::Arc;

// Chunks are a newtype wrapper over `Arc<Vec<u8>>`. The allow message parts to
// be immutably shared between different machines. It is useful in the interface
// for Message because it allows Message::new() and Message::with_header() to be
// polymorphic over a variety of message data sources. The various From impls
// makes this work.

/// A piece of a [Message](super::Message), either a message body or a
/// header.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) bytes: Arc<Vec<u8>>,
}

impl Chunk {
    /// Returns a new chunk containing the given bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            start: 0,
            end: bytes.len(),
            bytes: Arc::new(bytes),
        }
    }

    /// Returns the underlying bytes as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[self.start..self.end]
    }

    /// The number of bytes in the chunk.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Whether the chunk contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl PartialEq for Chunk {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl From<Vec<u8>> for Chunk {
    fn from(vector: Vec<u8>) -> Self {
        Self::new(vector)
    }
}

impl From<&[u8]> for Chunk {
    fn from(slice: &[u8]) -> Self {
        slice.to_vec().into()
    }
}

impl<const N: usize> From<&[u8; N]> for Chunk {
    fn from(array: &[u8; N]) -> Self {
        array.as_slice().into()
    }
}

impl<const N: usize> From<[u8; N]> for Chunk {
    fn from(array: [u8; N]) -> Self {
        array.as_slice().into()
    }
}

impl From<&str> for Chunk {
    fn from(string: &str) -> Self {
        string.as_bytes().into()
    }
}
