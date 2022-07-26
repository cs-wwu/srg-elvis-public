use std::rc::Rc;

// Chunks are a newtype wrapper over `Rc<Vec<u8>>`. The allow message parts to
// be immutably shared between different machines. It is useful in the interface
// for Message because it allows Message::new() and Message::with_header() to be
// polymorphic over a variety of message data sources. The various From impls
// makes this work.

/// A piece of a [Message](super::Message), either a message body or a
/// header.
#[derive(Debug)]
pub struct Chunk(Rc<Vec<u8>>);

impl Chunk {
    /// Returns a new chunk containing the given bytes.
    pub fn new(data: Vec<u8>) -> Self {
        Self(Rc::new(data))
    }

    /// Returns the underlying bytes as slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl From<Vec<u8>> for Chunk {
    fn from(vector: Vec<u8>) -> Self {
        Self(Rc::new(vector))
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

impl From<&str> for Chunk {
    fn from(string: &str) -> Self {
        string.as_bytes().into()
    }
}
