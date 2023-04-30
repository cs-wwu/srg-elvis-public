use crate::Message;
use std::cmp::Ordering;

/// A piece of a datagram being assembled. This type is set up to be sorted by
/// the fragment offset, thus allowing easy final reassembly of the collected fragments.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The bytes of the fragment
    message: Message,
    /// The fragment offset of this fragment
    offset: u16,
}

impl Fragment {
    /// Creates a new fragment
    pub fn new(message: Message, offset: u16) -> Self {
        Self { message, offset }
    }

    pub fn into_message(self) -> Message {
        self.message
    }
}

impl PartialEq for Fragment {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

impl Eq for Fragment {}

impl PartialOrd for Fragment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fragment {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset).reverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_order() {
        let a = Fragment::new(Message::default(), 0);
        let b = Fragment::new(Message::default(), 10);
        assert!(a > b);
    }
}
