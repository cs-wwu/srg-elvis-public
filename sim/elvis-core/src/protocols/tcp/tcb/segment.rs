use super::modular_cmp::mod_lt;
use crate::{protocols::tcp::tcp_parsing::TcpHeader, Message};
use std::cmp::Ordering;

/// A TCP segment consisting of
///
/// - The header
/// - The rest of the segment, known as the segment text
///
/// Importantly, this type implements ordering such that a priority queue will
/// pop segments with lower sequence numbers first. This allows segments to be
/// processed in sequence number order after they arrive.
#[derive(Debug, Clone)]
pub struct Segment {
    /// The TCP header
    pub header: TcpHeader,
    /// The data bytes carried by the segment
    pub text: Message,
}

impl Segment {
    /// Create a new segment
    pub fn new(seg: TcpHeader, message: Message) -> Self {
        Self {
            header: seg,
            text: message,
        }
    }

    /// The length of the segment data, including any control bits
    pub fn seg_len(&self) -> usize {
        self.text.len() + self.header.ctl.syn() as usize + self.header.ctl.fin() as usize
    }

    /// Get the header and message from the segment
    pub fn into_inner(self) -> (TcpHeader, Message) {
        (self.header, self.text)
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.header.seq == other.header.seq
    }
}

impl Eq for Segment {}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.header.seq == other.header.seq {
            Ordering::Equal
        } else if mod_lt(self.header.seq, other.header.seq) {
            // Reversing the order so the the priority queue handles messages
            // starting from lower sequence numbers
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}
