use crate::core::Buf;

/// A Message is an immutable set of bytes that are stored in one or more
/// contiguous blocks of memory.  A Message is created with Message::new.
/// The body and headers are pushed on to the message, creating a new immutable
/// message each time.
/// ```
/// use sim::core::{Buf, Message};
///
/// let mut message = Message::new();
/// let body = Buf::new(b"Body Data");
/// message = message.push(&body);
/// let header = Buf::new(b"Header");
/// message = message.push(&header)
/// ```
/// The underlying iumplementation uses reference counted Bytes in a vector of Bytes.
/// Pushing data onto a message is a zero-copy operation of the underlying data.
#[derive(Debug)]
pub struct Message {
    chunks: Vec<Buf>,
}

impl Message {
    /// Create a new empty message
    pub fn new() -> Message {
        Message {
            chunks: vec![]
        }
    }

    /// Return the length in bytes of the Message.
    /// This method sums up the length of each of the constituent chunks of the message.
    pub fn len(&self) -> usize {
        let mut result = 0;
        for chunk in &self.chunks {
            result += chunk.len();
        }
        result
    }

    /// Push the data of Bytes onto this message. The data is logically prepended
    /// to the front of the message. This is a zero copy operation.
    ///
    /// # Arguments
    ///
    /// * `data` - A Bytes reference that holds the data
    ///
    /// # Returns
    ///
    /// A new Message object with the header prepended. The original Message is unchanged
    pub fn push(&self, data: &Buf) -> Message {
        let mut copy = self.chunks.clone();
        copy.insert(0, data.clone());
        Message {
            chunks: copy
        }
    }

    /// Pop the size in bytes of data off the front of this Message.
    /// This is a zero copy operation.
    ///
    /// # Arguments
    ///
    /// * `size` - The size in bytes to remove from the front
    ///
    /// # Returns
    ///
    /// A new message with the header truncated by the given number of bytes.
    /// The original message is unchanged.
    pub fn pop(&self, size: usize) -> Message {
        let mut copy = self.chunks.clone();
        let mut remaining = size;
        while remaining > 0 {
            if copy[0].len() <= remaining {
                remaining -= copy[0].len();
                copy.remove(0);
            } else {
                copy[0] = copy[0].slice(remaining, copy[0].len());
                break;
            }
        }
        Message {
            chunks: copy,
        }
    }

    /// Return a reference to the constituent vector of chunk of bytes that make up
    /// this Message. The lifetime of the reference is bound to the lifetime of this Message.
    pub fn chunks<'a>(&'a self) -> &'a Vec<Buf> {
        &self.chunks
    }

    /// Read and copy data from the Message into the given buffer, up to the
    /// length of the buffer.
    ///
    /// # Arguments
    ///
    /// * buf - The byte buffer to read into
    ///
    /// # Returns
    ///
    /// The size in bytes copied into the buffer, or negative for error codes
    pub fn read(&self, _buf: &mut [u8]) -> isize {
        // TODO(seemong): Finish implementing the read
        // Open issue -- should we add an stream interface that advances the read pointer?
        0
    }
}
