use std::{
    fmt::Display,
    ops::{Index, Range},
    rc::Rc,
};

// Todo: Add support for appending messages
// Todo: Remove pop support
// Todo: Use indexing for slices

#[derive(Debug, Clone)]
pub struct Message {
    /// A message with headers
    stack: Rc<WrappedMessage>,
}

impl Message {
    /// Creates a new message with the given body content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::Message;
    /// let message = Message::new(b"Body");
    /// ```
    pub fn new(body: impl Into<Chunk>) -> Self {
        Self {
            stack: Rc::new(WrappedMessage::Body(body.into())),
        }
    }

    /// Creates a new message with the given header prepended.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn with_header(&self, header: impl Into<Chunk>) -> Self {
        Self {
            stack: Rc::new(WrappedMessage::Header(header.into(), self.stack.clone())),
        }
    }

    /// Creates a slice of the message from `start` to `end`. `start` is
    /// inclusive, `end` is exclusive. Using `None` for the start or the end
    /// creates an open-ended slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use elvis::core::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let sliced = message.slice(3, 8);
    /// assert!(sliced.iter().eq(b"derBo".iter().cloned()));
    /// let sliced = message.slice(None, 8);
    /// assert!(sliced.iter().eq(b"HeaderBo".iter().cloned()));
    /// let sliced = message.slice(3, None);
    /// assert!(sliced.iter().eq(b"derBody".iter().cloned()));
    /// ```
    pub fn slice(&self, start: impl Into<Option<usize>>, end: impl Into<Option<usize>>) -> Self {
        let start = start.into().unwrap_or(0);
        let end = end.into().unwrap_or(usize::MAX);
        Self {
            stack: Rc::new(WrappedMessage::Slice {
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
    /// # use elvis::core::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header");
    /// let expected = b"HeaderBody";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = u8> {
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

/// A cons list of message parts.
#[derive(Debug, Clone)]
enum WrappedMessage {
    Slice {
        start: usize,
        length: usize,
        message: Rc<WrappedMessage>,
    },
    Header(Chunk, Rc<WrappedMessage>),
    Body(Chunk),
}

/// An iterator over the bytes of a message
struct MessageBytes {
    /// Tracks the current message part
    stack: Option<Rc<WrappedMessage>>,
    /// Tracks the index into the current chunk
    i: usize,
    /// The length of the slice
    length: usize,
}

impl MessageBytes {
    /// Returns a new message bytes iterator.
    pub fn new(stack: Rc<WrappedMessage>) -> Self {
        Self {
            stack: Some(stack),
            i: 0,
            length: usize::MAX,
        }
    }
}

impl Iterator for MessageBytes {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.stack {
            Some(stack) => match stack.as_ref() {
                WrappedMessage::Slice {
                    start,
                    length,
                    message,
                } => {
                    self.i += start;
                    self.length = self.length.min(*length);
                    self.stack = Some(message.clone());
                    self.next()
                }
                WrappedMessage::Header(chunk, message) => {
                    if self.length > 0 {
                        match chunk.as_slice().get(self.i) {
                            Some(&byte) => {
                                self.i += 1;
                                self.length -= 1;
                                Some(byte)
                            }
                            None => {
                                self.i -= chunk.as_slice().len();
                                self.stack = Some(message.clone());
                                self.next()
                            }
                        }
                    } else {
                        None
                    }
                }
                WrappedMessage::Body(chunk) => {
                    if self.length > 0 {
                        let out = chunk.as_slice().get(self.i).cloned();
                        self.i += 1;
                        self.length -= 1;
                        out
                    } else {
                        None
                    }
                }
            },
            None => None,
        }
    }
}

/// A piece of a [Message](crate::core::Message), either a message body or a
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
        Self(Rc::new(slice.to_vec()))
    }
}

impl<const N: usize> From<&[u8; N]> for Chunk {
    fn from(array: &[u8; N]) -> Self {
        From::from(array.as_slice())
    }
}
