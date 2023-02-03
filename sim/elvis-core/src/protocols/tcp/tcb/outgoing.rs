use super::Segment;
use crate::{protocols::tcp::tcp_parsing::TcpHeader, Message};
use std::collections::VecDeque;

/// A collection of queues used for outgoing segments in TCP
#[derive(Debug, Clone, Default)]
pub struct Outgoing {
    /// Data bytes queued for transmission but not yet segmentized
    pub text: VecDeque<Message>,
    /// The retransmission queue. Contains segments that may need to be
    /// retransmitted.
    pub retransmit: VecDeque<Transmit>,
    /// Contains segments that should not be retransmitted, such as pure-ACK
    /// segments.
    pub oneshot: Vec<TcpHeader>,
}

impl Outgoing {
    /// The number of bytes of data currently queued for delivery in the
    /// retransmission queue
    pub fn queued_bytes(&self) -> usize {
        self.retransmit
            .iter()
            .map(|transmit| transmit.segment.text.len())
            .sum()
    }
}

/// A segment on the retransmission queue. Records whether the segment is due
/// for retransmission.
#[derive(Debug, Clone)]
pub struct Transmit {
    /// The segment
    pub segment: Segment,
    /// Whether the segment should be retransmitted. Reset whenever the
    /// retransmit timer runs out.
    pub needs_transmit: bool,
}

impl Transmit {
    /// Create a new Transmit
    pub fn new(segment: Segment) -> Self {
        Self {
            segment,
            needs_transmit: true,
        }
    }
}
