use super::Chunk;
use std::{
    collections::{vec_deque, VecDeque},
    slice,
};

/// An iterator over the bytes of a message
pub struct MessageBytes<'a> {
    chunks: vec_deque::Iter<'a, Chunk>,
    current: slice::Iter<'a, u8>,
}

impl<'a> MessageBytes<'a> {
    pub(super) fn new(chunks: &'a VecDeque<Chunk>) -> Self {
        let mut chunks = chunks.iter();
        Self {
            current: chunks
                .next()
                .map(|chunk| chunk.as_slice().iter())
                .unwrap_or([].iter()),
            chunks,
        }
    }
}

impl<'a> Iterator for MessageBytes<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current.next() {
            Some(byte) => Some(*byte),
            None => {
                self.current = self
                    .chunks
                    .next()
                    .map(|chunk| chunk.as_slice().iter())
                    .unwrap_or([].iter());
                self.current.next().map(|byte| *byte)
            }
        }
    }
}
