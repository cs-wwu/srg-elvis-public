use super::WrappedMessage;
use std::sync::Arc;

/// An iterator over the bytes of a message
pub struct MessageBytes {
    /// Tracks the current message part
    stack: Arc<WrappedMessage>,
    /// Tracks the index into the current chunk
    i: usize,
    /// The length of the slice
    length: usize,
}

impl MessageBytes {
    pub(super) fn new(stack: Arc<WrappedMessage>, start: usize, length: usize) -> Self {
        Self {
            stack,
            i: start,
            length,
        }
    }
}

impl Iterator for MessageBytes {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.length == 0 {
            return None;
        }

        match self.stack.as_ref() {
            WrappedMessage::Header(chunk, rest) => match chunk.as_slice().get(self.i) {
                Some(byte) => {
                    self.i += 1;
                    self.length -= 1;
                    Some(*byte)
                }
                None => {
                    self.i = 0;
                    self.stack = rest.clone();
                    self.next()
                }
            },

            WrappedMessage::Body(chunk) => {
                let out = chunk.as_slice().get(self.i).cloned();
                self.i += 1;
                self.length -= 1;
                out
            }
        }
    }
}
