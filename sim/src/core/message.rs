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
    /// # use sim::core::{Message, Chunk};
    /// let message = Message::new(Chunk::from_slice(&[1, 2, 3]));
    /// ```
    pub fn new(body: Chunk) -> Self {
        Self {
            stack: Rc::new(WrappedMessage::Body(body)),
        }
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self::new(slice.into())
    }

    /// Creates a new message with the given header prepended.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::{Message, Chunk};
    /// let message = Message::from_slice(&[4, 5, 6]);
    /// let message = message.push(Chunk::from_slice(&[1, 2, 3]));
    /// let expected = &[1u8, 2, 3, 4, 5, 6];
    /// for (actual, &expected) in message.iter_bytes().zip(expected.iter()) {
    ///     assert_eq!(actual, expected);
    /// }
    /// ```
    pub fn push(&self, header: Chunk) -> Self {
        Self {
            stack: Rc::new(WrappedMessage::Header(header, self.stack.clone())),
        }
    }

    pub fn push_slice(&self, header: &[u8]) -> Self {
        self.push(header.into())
    }

    /// Create a message from a list of chunks. The slice should have outermost
    /// headers first and the body last.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::Message;
    /// let slices = &[
    ///     &[1u8, 2, 3][..],
    ///     &[4, 5, 6][..],
    /// ];
    /// let message = Message::from_slices(slices).unwrap();
    /// let body = message.pop().unwrap();
    /// for (actual, &expected) in body.iter_bytes().zip(slices[1].iter()) {
    ///     assert_eq!(actual, expected);
    /// }
    /// ```
    pub fn from_slices(chunks: &[&[u8]]) -> Option<Self> {
        let mut iter = chunks.iter().rev();
        match iter.next() {
            Some(&first_chunk) => {
                let mut message = Self::new(first_chunk.into());
                for &chunk in iter {
                    message = message.push(chunk.into());
                }
                Some(message)
            }
            None => None,
        }
    }

    /// Returns the outmost header from the message and the remainder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::Message;
    /// let slices = &[
    ///     &[1u8, 2, 3][..],
    ///     &[4, 5, 6][..],
    ///     &[7, 8, 9][..],
    /// ];
    /// let message = Message::from_slices(slices).unwrap();
    /// let body = message.pop().unwrap();
    /// let expected = &[4u8, 5, 6, 7, 8, 9][..];
    /// for (actual, &expected) in body.iter_bytes().zip(slices[1..].iter().flat_map(|slice| slice.iter())) {
    ///     assert_eq!(actual, expected);
    /// }
    /// ```
    pub fn pop(&self) -> Option<Message> {
        match self.stack.as_ref() {
            WrappedMessage::Header(_, message) => Some(Self {
                stack: message.clone(),
            }),
            WrappedMessage::Body(_) => None,
        }
    }

    /// Returns an iterator over the bytes of the entire message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::Message;
    /// let slices = &[
    ///     &[1u8, 2, 3][..],
    ///     &[4, 5, 6][..],
    /// ];
    /// let message = Message::from_slices(slices).unwrap();
    /// let expected = &[1u8, 2, 3, 4, 5, 6];
    /// for (actual, &expected) in message.iter_bytes().zip(expected.iter()) {
    ///     assert_eq!(actual, expected);
    /// }
    /// ```
    pub fn iter_bytes(&self) -> impl Iterator<Item = u8> {
        self.iter_chunks().flat_map(|chunk| chunk.iter())
    }

    /// Returns an iterator over the chunks of the message. Chunks can be body
    /// content or headers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sim::core::Message;
    /// let slices = &[
    ///     &[1u8, 2, 3][..],
    ///     &[4, 5, 6][..],
    /// ];
    /// let message = Message::from_slices(slices).unwrap();
    /// for (chunk, &slice) in message.iter_chunks().zip(slices.iter()) {
    ///     assert_eq!(chunk.as_slice(), slice);
    /// }
    /// ```
    pub fn iter_chunks(&self) -> impl Iterator<Item = Chunk> {
        MessageChunks::new(self.stack.clone())
    }

    pub fn len(&self) -> usize {
        self.iter_bytes().count()
    }
}

/// A cons list of message parts.
#[derive(Debug, Clone)]
enum WrappedMessage {
    Header(Chunk, Rc<WrappedMessage>),
    Body(Chunk),
}

impl WrappedMessage {
    /// Returns the outermost chunk of the message.
    pub fn get_outer(&self) -> &Chunk {
        match self {
            WrappedMessage::Header(chunk, _) => chunk,
            WrappedMessage::Body(chunk) => chunk,
        }
    }
}

/// An iterator over message chunks.
pub struct MessageChunks {
    stack: Option<Rc<WrappedMessage>>,
}

impl MessageChunks {
    /// Creates a new iterator.
    fn new(stack: Rc<WrappedMessage>) -> Self {
        Self { stack: Some(stack) }
    }
}

impl Iterator for MessageChunks {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.clone() {
            Some(stack) => {
                let out = stack.get_outer();
                self.stack = match stack.as_ref() {
                    WrappedMessage::Header(_, wrapped) => Some(wrapped.clone()),
                    WrappedMessage::Body(_) => None,
                };
                Some(out.clone())
            }
            None => None,
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.iter_bytes() {
            write!(f, "{:x} ", byte)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Chunk(Rc<Vec<u8>>);

impl Chunk {
    pub fn new(data: Vec<u8>) -> Self {
        Self(Rc::new(data))
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self::new(slice.to_vec())
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> {
        ChunkBytes::new(self.clone())
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl From<&[u8]> for Chunk {
    fn from(slice: &[u8]) -> Self {
        Self(Rc::new(slice.to_vec()))
    }
}

struct ChunkBytes {
    chunk: Chunk,
    i: usize,
}

impl ChunkBytes {
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
