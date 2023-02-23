use super::Chunk;
use std::{collections::VecDeque, slice};

/// An iterator over the bytes of a message
pub struct MessageBytes<'a> {
    chunks: &'a VecDeque<Chunk>,
    current: Option<slice::Iter<'a, u8>>,
    chunk_i: usize,
}

impl<'a> MessageBytes<'a> {
    pub(super) fn new(chunks: &'a VecDeque<Chunk>) -> Self {
        Self {
            chunks,
            current: Some([].iter()),
            chunk_i: 0,
        }
    }
}

impl<'a> Iterator for MessageBytes<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.as_mut()?;
        match current.next() {
            Some(byte) => Some(*byte),
            None => {
                self.current = self
                    .chunks
                    .get(self.chunk_i)
                    .map(|chunk| chunk.as_slice().iter());
                self.chunk_i += 1;
                self.next()
            }
        }
    }
}
