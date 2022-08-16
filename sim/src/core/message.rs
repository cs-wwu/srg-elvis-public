//! Byte collections with efficient operations for protocols.
//!
//! This module primarily implements the [`Message`] collection.

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
    start: usize,
    end: usize,
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
            start: 0,
            end: body.len(),
            stack: Arc::new(WrappedMessage::Body(body)),
        }
    }

    /// Creates a new message with the given header prepended.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.prepend(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn prepend(&mut self, header: impl Into<Chunk>) {
        self.prepend_inner(header.into());
    }

    fn prepend_inner(&mut self, header: Chunk) {
        self.end += header.len();
        self.stack = Arc::new(WrappedMessage::Header(header, self.stack.clone()));
    }

    /// Creates a slice of the message for the given range. All Rust range types
    /// defined in std::ops are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.prepend(b"Header");
    /// message.slice(3..8);
    /// assert!(message.iter().eq(b"derBo".iter().cloned()));
    /// ```
    pub fn slice(&mut self, range: impl Into<SliceRange>) {
        self.slice_inner(range.into())
    }

    fn slice_inner(&mut self, range: SliceRange) {
        let (start, len) = range.start_and_len();
        assert!(start + len.unwrap_or(0) <= self.len());
        self.start += start;
        if let Some(len) = len {
            self.end = self.start + len;
        }
        loop {
            match self.stack.as_ref() {
                WrappedMessage::Header(chunk, rest) => {
                    let len = chunk.len();
                    if self.start >= len {
                        self.start -= len;
                        self.end -= len;
                        self.stack = rest.clone();
                    } else {
                        break;
                    }
                }
                WrappedMessage::Body(_) => break,
            }
        }
    }

    fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns an iterator over the bytes of the entire message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.prepend(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn iter(&self) -> MessageBytes {
        MessageBytes::new(self.stack.clone(), self.start, self.len())
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
    Header(Chunk, Arc<WrappedMessage>),
    Body(Chunk),
}
