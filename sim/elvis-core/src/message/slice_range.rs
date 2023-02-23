use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// A generalization of Rust's range types for use with message slicing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SliceRange {
    pub start: usize,
    pub len: Option<usize>,
}

impl From<Range<usize>> for SliceRange {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            len: Some(range.len()),
        }
    }
}

impl From<RangeFrom<usize>> for SliceRange {
    fn from(range: RangeFrom<usize>) -> Self {
        Self {
            start: range.start,
            len: None,
        }
    }
}

impl From<RangeFull> for SliceRange {
    fn from(_: RangeFull) -> Self {
        Self {
            start: 0,
            len: None,
        }
    }
}

impl From<RangeInclusive<usize>> for SliceRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        Self {
            start: *range.start(),
            len: Some(range.end() + 1 - range.start()),
        }
    }
}

impl From<RangeTo<usize>> for SliceRange {
    fn from(range: RangeTo<usize>) -> Self {
        Self {
            start: 0,
            len: Some(range.end),
        }
    }
}

impl From<RangeToInclusive<usize>> for SliceRange {
    fn from(range: RangeToInclusive<usize>) -> Self {
        Self {
            start: 0,
            len: Some(range.end + 1),
        }
    }
}
