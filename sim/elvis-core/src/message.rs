//! Byte collections with efficient operations for protocols.
//!
//! This module primarily implements the [`Message`] collection.

use std::{collections::VecDeque, fmt::Display};

mod chunk;
pub use chunk::Chunk;

mod slice_range;
use slice_range::SliceRange;

mod message_bytes;
pub use message_bytes::MessageBytes;

/// A byte collection with efficient operations for implementing protocols.
///
/// When writing a networking protocol, it is standard to append headers, remove
/// headers, and concatenate pieces of a message. These operations should be as
/// fast as possible. In particular, we want to avoid copying bytes wherever
/// possible. A message provides these capabilities and serves as a container
/// for composing, sending, and splitting byte sequences.
#[derive(Debug, Clone, Default)]
pub struct Message {
    chunks: VecDeque<Chunk>,
    len: usize,
}

impl Message {
    /// Creates a new message with the given body content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::message::Message;
    /// let message = Message::new(b"Body");
    /// ```
    pub fn new(body: impl Into<Chunk>) -> Self {
        Self::new_inner(body.into())
    }

    fn new_inner(body: Chunk) -> Self {
        let len = body.len();
        let mut chunks = VecDeque::new();
        chunks.push_back(body);
        Self { chunks, len }
    }

    /// Creates a new message with the given header prepended.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn header(&mut self, header: impl Into<Chunk>) {
        self.header_inner(header.into());
    }

    fn header_inner(&mut self, header: Chunk) {
        self.len += header.len();
        self.chunks.push_front(header);
    }

    /// Adds the given message to the end of this one.
    pub fn concatenate(&mut self, other: Message) {
        self.len += other.len;
        self.chunks.extend(other.chunks.into_iter());
    }

    /// Creates a slice of the message for the given range. All Rust range types
    /// defined in std::ops are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.header(b"Header");
    /// message.slice(3..8);
    /// assert!(message.iter().eq(b"derBo".iter().cloned()));
    /// ```
    pub fn slice(&mut self, range: impl Into<SliceRange>) {
        self.slice_inner(range.into())
    }

    fn slice_inner(&mut self, range: SliceRange) {
        let SliceRange { mut start, len } = range;
        assert!(start + len.unwrap_or(0) <= self.len());
        self.len = len.unwrap_or(self.len - start);

        // Remove leading chunks that are no longer accessible
        while let Some(head) = self.chunks.front() {
            let head_len = head.len();
            if head_len <= start {
                start -= head_len;
                self.chunks.pop_front();
            } else {
                break;
            }
        }

        // Update the start of the first chunk
        if let Some(head) = self.chunks.front_mut() {
            head.start += start;
        }

        // Find and update the last accessible chunk
        let mut bytes_to_keep = self.len;
        let mut i = 0;
        for chunk in self.chunks.iter_mut() {
            i += 1;
            let chunk_len = chunk.len();
            if bytes_to_keep >= chunk_len {
                bytes_to_keep -= chunk_len;
            } else {
                chunk.end = chunk.start + bytes_to_keep;
                break;
            }
        }

        // Remove inaccessible chunks from the end
        self.chunks.drain(i..);
    }

    /// Removes the first `len` bytes from the message and returns them as a new
    /// message.
    pub fn cut(&mut self, len: usize) -> Self {
        assert!(len <= self.len);
        self.len -= len;

        let mut chunks = VecDeque::new();
        let mut to_remove = len;

        // Remove leading chunks that are no longer accessible
        while let Some(mut head) = self.chunks.pop_front() {
            let head_len = head.len();
            if head_len <= to_remove {
                to_remove -= head_len;
                chunks.push_back(head);
            } else {
                if to_remove > 0 {
                    let mut head = head.clone();
                    head.end = head.start + to_remove;
                    chunks.push_back(head);
                }
                head.start += to_remove;
                self.chunks.push_front(head);
                break;
            }
        }

        Self { chunks, len }
    }

    pub fn remove_front(&mut self, len: usize) {
        assert!(len <= self.len);
        self.len -= len;

        let mut to_remove = len;

        // Remove leading chunks that are no longer accessible
        while let Some(head) = self.chunks.front_mut() {
            let head_len = head.len();
            if head_len <= to_remove {
                to_remove -= head_len;
                self.chunks.pop_front();
            } else {
                head.start += to_remove;
                break;
            }
        }
    }

    /// The length of the message.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the message contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the bytes of the entire message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis_core::message::{Message, Chunk};
    /// let mut message = Message::new(b"Body");
    /// message.header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn iter(&self) -> MessageBytes {
        MessageBytes::new(&self.chunks)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.iter().collect()
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.iter() {
            write!(f, "{byte:x} ")?;
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

impl From<Vec<u8>> for Message {
    fn from(val: Vec<u8>) -> Self {
        Message::new(val)
    }
}

impl From<&[u8]> for Message {
    fn from(val: &[u8]) -> Self {
        Message::new(val)
    }
}

impl<const L: usize> From<[u8; L]> for Message {
    fn from(val: [u8; L]) -> Self {
        Message::new(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_message() {
        let body = b"body";
        let message = Message::new(body);
        assert_eq!(message.len(), body.len());
        assert_eq!(&message.to_vec(), body);
    }

    #[test]
    fn slicing() {
        let mut message = Message::new("body");
        message.slice(2..);
        let expected = b"dy";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn multi_slice() {
        let mut message = Message::new(b"Things and stuff");
        message.slice(1..15);
        message.slice(1..13);
        let expected = b"ings and stu";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn header() {
        let mut message = Message::new(b"body");
        message.header("header");
        let expected = b"headerbody";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn multi_slice_with_header() {
        let mut message = Message::new(b"Body");
        message.header(b"Header");
        message.slice(3..8);
        message.slice(2..4);
        let expected = b"rB";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn mixed_operations() {
        let mut message = Message::new(b"Hello, world");
        message.slice(0..5);
        message.header(b"Header");
        message.slice(3..8);
        let expected = b"derHe";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn sliced_chunk() {
        let mut message = Message::new(b"Hello, world");
        message.slice(7..);
        message.header(b"Header ");
        let expected = b"Header world";
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn remove_headers() {
        let expected = b"body";
        let mut message = Message::new(expected);
        message.header(b"ipv4");
        message.header(b"tcp");
        message.slice(3..);
        message.slice(4..);
        assert_eq!(message.len(), expected.len());
        assert_eq!(&message.to_vec(), expected);
    }

    #[test]
    fn slice_everything_1() {
        let mut message = Message::new(b"body");
        message.slice(4..);
        assert_eq!(message.len(), 0);
        assert_eq!(&message.to_vec(), &[]);
    }

    #[test]
    fn slice_everything_2() {
        let mut message = Message::new(b"body");
        message.slice(..0);
        assert_eq!(message.len(), 0);
        assert_eq!(&message.to_vec(), &[]);
    }

    #[test]
    fn slice_then_prepend_and_pop() {
        let mut message = Message::new(b"large message");
        message.slice(6..);
        assert_eq!(message.len(), 7);
        assert!(message.iter().eq(b"message".iter().cloned()));
        message.header(b"header");
        assert_eq!(message.len(), 13);
        assert!(message.iter().eq(b"headermessage".iter().cloned()));
        message.slice(6..);
        assert_eq!(message.len(), 7);
        assert_eq!(&message.to_vec(), b"message");
    }

    #[test]
    fn concatenate() {
        let mut message = Message::new("Hello");
        message.concatenate(Message::new(" world!"));
        assert_eq!(&message.to_vec(), b"Hello world!");
    }

    #[test]
    fn empty_message() {
        let message = Message::new("");
        assert_eq!(&message.to_vec(), b"");
    }

    #[test]
    fn cut() {
        let mut a = Message::new("Hello, world");
        let b = a.cut(5);
        assert_eq!(a, Message::new(", world"));
        assert_eq!(b, Message::new("Hello"));
    }

    #[test]
    fn cut_more_complex() {
        let mut a = Message::new("stuffa");
        a.header(" and ");
        a.header("athings");
        a.slice(1..);
        a.slice(..16);
        let b = a.cut(10);
        assert_eq!(a, Message::new(" stuff"));
        assert_eq!(b, Message::new("things and"));
    }

    #[test]
    fn remove_front() {
        let mut a = Message::new("Hello, world");
        a.remove_front(5);
        assert_eq!(a, Message::new(", world"));
    }

    #[test]
    fn remove_front_more_complex() {
        let mut a = Message::new("stuffa");
        a.header(" and ");
        a.header("athings");
        a.slice(1..);
        a.slice(..16);
        a.remove_front(10);
        assert_eq!(a, Message::new(" stuff"));
    }
}
