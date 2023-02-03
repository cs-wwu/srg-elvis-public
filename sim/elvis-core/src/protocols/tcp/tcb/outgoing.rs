use super::Transmit;
use crate::{protocols::tcp::tcp_parsing::TcpHeader, Message};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct Outgoing {
    /// Bytes already gobbled from the front of the first message in `text`.
    pub text: VecDeque<Message>,
    pub retransmit: VecDeque<Transmit>,
    pub oneshot: Vec<TcpHeader>,
}

impl Outgoing {
    pub fn queued_bytes(&self) -> usize {
        self.retransmit
            .iter()
            .map(|transmit| transmit.segment.text.len())
            .sum()
    }
}
