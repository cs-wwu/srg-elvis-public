use super::Segment;
use crate::{protocols::tcp::tcp_parsing::TcpHeader, Message};
use std::{collections::VecDeque, sync::RwLock};

/// A collection of queues used for outgoing segments in TCP
#[derive(Debug, Default)]
pub struct Outgoing {
    /// Data bytes queued for transmission but not yet segmentized
    pub text: RwLock<Message>,
    /// The retransmission queue. Contains segments that may need to be
    /// retransmitted.
    pub retransmit: RwLock<VecDeque<Transmit>>,
    /// Contains segments that should not be retransmitted, such as pure-ACK
    /// segments.
    pub oneshot: RwLock<Vec<TcpHeader>>,
}

impl Outgoing {
    /// The number of bytes of data currently queued for delivery in the
    /// retransmission queue
    pub fn queued_bytes(&self) -> usize {
        self.retransmit
            .read()
            .unwrap()
            .iter()
            .map(|transmit| transmit.segment.text.len())
            .sum()
    }

    pub fn reset(&self) {
        *self.text.write().unwrap() = Default::default();
        *self.retransmit.write().unwrap() = Default::default();
        *self.oneshot.write().unwrap() = Default::default();
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
