use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// A generalization of Rust's range types for use with message slicing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SliceRange {
    Range(Range<usize>),
    RangeFrom(RangeFrom<usize>),
    RangeFull(RangeFull),
    RangeInclusive(RangeInclusive<usize>),
    RangeTo(RangeTo<usize>),
    RangeToInclusive(RangeToInclusive<usize>),
}

impl SliceRange {
    pub fn start_and_len(&self) -> (usize, Option<usize>) {
        use SliceRange::*;
        match self {
            Range(range) => (range.start, Some(range.len())),
            RangeFrom(range) => (range.start, None),
            RangeFull(_) => (0, None),
            RangeInclusive(range) => (*range.start(), Some(range.end() + 1 - range.start())),
            RangeTo(range) => (0, Some(range.end)),
            RangeToInclusive(range) => (0, Some(range.end + 1)),
        }
    }
}

impl From<Range<usize>> for SliceRange {
    fn from(range: Range<usize>) -> Self {
        Self::Range(range)
    }
}

impl From<RangeFrom<usize>> for SliceRange {
    fn from(range: RangeFrom<usize>) -> Self {
        Self::RangeFrom(range)
    }
}

impl From<RangeFull> for SliceRange {
    fn from(range: RangeFull) -> Self {
        Self::RangeFull(range)
    }
}

impl From<RangeInclusive<usize>> for SliceRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        Self::RangeInclusive(range)
    }
}

impl From<RangeTo<usize>> for SliceRange {
    fn from(range: RangeTo<usize>) -> Self {
        Self::RangeTo(range)
    }
}

impl From<RangeToInclusive<usize>> for SliceRange {
    fn from(range: RangeToInclusive<usize>) -> Self {
        Self::RangeToInclusive(range)
    }
}
