use std::{fmt::Display, rc::Rc};

/// A message with headers.
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
    /// # use sim::core::Message;
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
    /// # use sim::core::{Message, Chunk};
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
    /// inclusive, `end` is exclusive.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::{Message, Chunk};
    /// let message = Message::new(b"Body").with_header(b"Header").slice(3, 8);
    /// let expected = b"derBo";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            stack: Rc::new(WrappedMessage::Slice {
                start,
                end,
                message: self.stack.clone(),
            }),
        }
    }

    /// Returns the outmost header from the message and the remainder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::Message;
    /// let message = Message::new(b"Body").with_header(b"Header1").with_header(b"Header2");
    /// let message = message.pop().unwrap();
    /// let expected = b"Header1Body";
    /// assert!(message.iter().eq(expected.iter().cloned()));
    /// ```
    pub fn pop(&self) -> Option<Message> {
        match self.stack.as_ref() {
            WrappedMessage::Header(_, message) => Some(Self {
                stack: message.clone(),
            }),
            WrappedMessage::Body(_) => None,
            WrappedMessage::Slice {
                start: _,
                end: _,
                message,
            } => Some(Self {
                stack: message.clone(),
            }),
        }
    }

    /// Returns an iterator over the bytes of the entire message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::{Message, Chunk};
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
        end: usize,
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
    /// The byte to end on
    end: usize,
}

impl MessageBytes {
    pub fn new(stack: Rc<WrappedMessage>) -> Self {
        Self {
            stack: Some(stack),
            i: 0,
            end: usize::MAX,
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
                    end,
                    message,
                } => {
                    self.i += start;
                    self.end = match self.end {
                        usize::MAX => *end,
                        _ => self.end + *end,
                    };
                    self.stack = Some(message.clone());
                    self.next()
                }
                WrappedMessage::Header(chunk, message) => {
                    if self.i < self.end {
                        match chunk.as_slice().get(self.i) {
                            Some(&byte) => {
                                self.i += 1;
                                Some(byte)
                            }
                            None => {
                                self.i -= chunk.len();
                                self.end -= chunk.len();
                                self.stack = Some(message.clone());
                                self.next()
                            }
                        }
                    } else {
                        None
                    }
                }
                WrappedMessage::Body(chunk) => {
                    if self.i < self.end {
                        let out = chunk.as_slice().get(self.i).cloned();
                        self.i += 1;
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

#[derive(Debug)]
pub struct Chunk(Rc<Vec<u8>>);

impl Chunk {
    pub fn new(data: Vec<u8>) -> Self {
        Self(Rc::new(data))
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> {
        ChunkBytes::new(self.clone())
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.0.len()
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
        Self(Rc::new(array.to_vec()))
    }
}

/// An iterator over the bytes of a [Chunk](crate::core::Chunk).
struct ChunkBytes {
    /// The chunk to iterate over
    chunk: Chunk,
    /// The current index into the chunk
    i: usize,
}

impl ChunkBytes {
    /// Returns a new iterator for the chunk.
    pub fn new(chunk: Chunk) -> Self {
        Self { chunk, i: 0 }
    }
}

impl Iterator for ChunkBytes {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.i;
        self.i += 1;
        self.chunk.0.get(i).cloned()
    }
}
