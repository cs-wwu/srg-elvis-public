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
                .unwrap_or_else(|| [].iter()),
            chunks,
        }
    }
}

impl<'a> Iterator for MessageBytes<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.current
            .next()
            .or_else(|| {
                self.current = self
                    .chunks
                    .next()
                    .map(|chunk| chunk.as_slice().iter())
                    .unwrap_or_else(|| [].iter());
                self.current.next()
            })
            .cloned()
    }
}
