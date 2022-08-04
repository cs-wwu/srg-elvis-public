use std::sync::Arc;

use super::WrappedMessage;

/// An iterator over the bytes of a message
pub struct MessageBytes {
    /// Tracks the current message part
    stack: Option<Arc<WrappedMessage>>,
    /// Tracks the index into the current chunk
    i: usize,
    /// The length of the slice
    length: usize,
}

impl MessageBytes {
    pub(super) fn new(stack: Arc<WrappedMessage>) -> Self {
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
