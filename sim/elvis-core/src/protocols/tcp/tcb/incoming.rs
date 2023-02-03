use super::{modular_cmp::mod_le, Segment};
use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
pub struct Incoming(Segment);

impl Incoming {
    pub fn new(segment: Segment) -> Self {
        Self(segment)
    }

    pub fn into_inner(self) -> Segment {
        self.0
    }
}

impl PartialEq for Incoming {
    fn eq(&self, other: &Self) -> bool {
        self.0.header.seq == other.0.header.seq
    }
}

impl Eq for Incoming {}

impl PartialOrd for Incoming {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Incoming {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0.header.seq == other.0.header.seq {
            Ordering::Equal
        } else if mod_le(self.0.header.seq, other.0.header.seq) {
            // Reversing the order so the the priority queue handles messages
            // starting from lower sequence numbers
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

impl Deref for Incoming {
    type Target = Segment;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Incoming {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
