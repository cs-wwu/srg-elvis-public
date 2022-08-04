use std::{fmt::Display, sync::Arc};

mod chunk;
pub use chunk::Chunk;

mod slice_range;
use slice_range::SliceRange;

mod message_bytes;
pub use message_bytes::MessageBytes;

// TODO(hardint): Add support for appending messages
// TODO(hardint): Add support for incorrectly transmitted bytes
// TODO(hardint): Store length on the message

/// A byte collection with efficient operations for implementing protocols.
///
/// When writing a networking protocol, it is standard to append headers, remove
/// headers, and concatenate pieces of a message. These operations should be as
/// fast as possible. In particular, we want to avoid copying bytes wherever
/// possible. A message provides these capabilities and serves as a container
/// for composing, sending, and splitting byte sequences.
#[derive(Debug, Clone)]
pub struct Message {
    stack: Arc<WrappedMessage>,
}

impl Message {
    /// Creates a new message with the given body content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::Message;
    /// let message = Message::new(b"Body");
    /// ```
    pub fn new(body: impl Into<Chunk>) -> Self {
        Self::new_inner(body.into())
    }

    fn new_inner(body: Chunk) -> Self {
        Self {
            stack: Arc::new(WrappedMessage::Body(body)),
        }
    }

    /// Creates a new message with the given header prepended.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn with_header(&self, header: impl Into<Chunk>) -> Self {
        self.with_header_inner(header.into())
    }

    fn with_header_inner(&self, header: Chunk) -> Self {
        Self {
            stack: Arc::new(WrappedMessage::Header(header, self.stack.clone())),
        }
    }

    /// Creates a slice of the message for the given range. All Rust range types
    /// defined in std::ops are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let sliced = message.slice(3..8);
    /// assert!(sliced.iter().eq(b"derBo".iter().cloned()));
    /// let sliced = message.slice(..8);
    /// assert!(sliced.iter().eq(b"HeaderBo".iter().cloned()));
    /// let sliced = message.slice(3..);
    /// assert!(sliced.iter().eq(b"derBody".iter().cloned()));
    /// ```
    pub fn slice(&self, range: impl Into<SliceRange>) -> Self {
        self.slice_inner(range.into())
    }

    fn slice_inner(&self, range: SliceRange) -> Self {
        let start = range.start();
        let end = range.end();
        Self {
            stack: Arc::new(WrappedMessage::Slice {
                start,
                length: end - start,
                message: self.stack.clone(),
            }),
        }
    }

    /// Returns an iterator over the bytes of the entire message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn iter(&self) -> MessageBytes {
        MessageBytes::new(self.stack.clone())
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.iter() {
            write!(f, "{:x} ", byte)?;
        }
        Ok(())
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for Message {}

/// A cons list of message parts.
#[derive(Debug, Clone)]
enum WrappedMessage {
    Slice {
        start: usize,
        length: usize,
        message: Arc<WrappedMessage>,
    },
    Header(Chunk, Arc<WrappedMessage>),
    Body(Chunk),
}
