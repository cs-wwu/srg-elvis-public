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
    /// Returns the inclusive lower bound of the range.
    pub fn start(&self) -> usize {
        use SliceRange::*;
        match self {
            RangeFull(_) | RangeTo(_) | RangeToInclusive(_) => 0,
            Range(range) => range.start,
            RangeFrom(range) => range.start,
            RangeInclusive(range) => *range.start(),
        }
    }

    /// Returns the exclusive upper bound of the range.
    pub fn end(&self) -> usize {
        use SliceRange::*;
        match self {
            RangeFrom(_) | RangeFull(_) => usize::MAX,
            Range(range) => range.end,
            RangeTo(range) => range.end,
            RangeToInclusive(range) => range.end + 1,
            RangeInclusive(range) => range.end() + 1,
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
