//! Implements the reassembly procedure from RFC791, section 3.2, page 27: An Example
//! Reassembly Procedure
//! <https://www.rfc-editor.org/rfc/rfc791>

mod bitvec;
mod buf_id;
mod fragment;
mod segment;

use self::{buf_id::BufId, segment::Epoch};
use super::ipv4_parsing::Ipv4Header;
use crate::Message;
use rustc_hash::FxHashMap;
use segment::Segment;
use std::{collections::hash_map::Entry, time::Duration};

/// Manages the reassembly of fragmented IP packets.
#[derive(Debug, Default, Clone)]
pub struct Reassembly {
    /// Fragmented IP packets that are still waiting on fragments to become
    /// complete.
    segments: FxHashMap<BufId, Segment>,
}

impl Reassembly {
    /// Creates a new reassembly manager.
    #[allow(unused)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a received IP packet for reassembly. Either of the following may
    /// result:
    /// - The fragment completes a datagram and the complete datagram is
    /// returned
    /// - The fragment is part of an incomplete datagram and is buffered while
    ///   waiting for the rest of the datagram. See [`ReceivePacketResult`] for
    ///   more details on the caller's responsibility in this event.
    pub fn receive_packet(&mut self, header: Ipv4Header, body: Message) -> ReceivePacketResult {
        // (1) BUFID <- source|destination|protocol|identification
        let buf_id = BufId::from_header(&header);
        // (2) IF FO = 0 AND MF = 0
        if header.flags.is_last_fragment() && header.fragment_offset == 0 {
            // (3) THEN IF buffer with BUFID is allocated
            // (4) THEN flush all reassembly for this BUFID
            self.segments.remove(&buf_id);
            // (5) Submit datagram to next step
            return ReceivePacketResult::Complete(header, body);
        }

        // (6) ELSE IF no buffer with BUFID is allocated
        // (7) THEN allocate reassembly resources with BUFID; TIMER <- TLB; TDL <- 0;
        let segment = self.segments.entry(buf_id).or_insert(Segment::new());

        match segment.receive_packet(header, body) {
            Some((header, message)) => {
                // (16) free all reassembly resources
                self.segments.remove(&buf_id).unwrap();
                ReceivePacketResult::Complete(header, message)
            }
            None => {
                // (18) give up until next fragment or timer expires
                // (19) timer expires: flush all reassembly with this BUFID
                ReceivePacketResult::Incomplete(
                    Duration::from_secs(segment.timeout_seconds as u64),
                    buf_id,
                    segment.epoch,
                )
            }
        }
    }

    /// Removes the resources associated with the given [`BufId`] if no new
    /// fragments have arrived since the given epoch.
    pub fn maybe_cull_segment(&mut self, buf_id: BufId, epoch: Epoch) {
        match self.segments.entry(buf_id) {
            Entry::Occupied(pending) => {
                if pending.get().epoch == epoch {
                    pending.remove_entry();
                }
            }
            Entry::Vacant(_) => {}
        }
    }
}

/// The result of receiving an IP packet.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReceivePacketResult {
    /// The added fragment completed a datagram.
    Complete(Ipv4Header, Message),
    /// The added fragment did not complete a datagram. The caller should set a
    /// timeout for the given duration and call
    /// [`Reassembly::maybe_cull_segment`] with the provided [`BufId`] and
    /// [`Epoch`] after the timeout expires.
    Incomplete(Duration, BufId, Epoch),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::Mtu,
        protocols::ipv4::{
            fragmentation::{fragment, Fragments},
            test_header_builder::TestHeaderBuilder,
        },
    };

    #[test]
    fn reassemble_segments() {
        const LEN: u16 = 1000;
        const MTU: Mtu = 600;

        let bytes_a: Vec<_> = (0..LEN).map(|i| i as u8).collect();
        let expected_a = Message::new(bytes_a);

        let bytes_b: Vec<_> = (0..LEN).map(|i| (i as u8).wrapping_add(5)).collect();
        let expected_b = Message::new(bytes_b);

        let header_a = TestHeaderBuilder::new(LEN).build();
        let a = match fragment(header_a, expected_a.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let [a1, a2] = a.as_slice() else {
            panic!("Expected two fragments")
        };

        let header_b = TestHeaderBuilder::new(LEN).identification(420).build();
        let b = match fragment(header_b, expected_b.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let [b1, b2] = b.as_slice() else {
            panic!("Expected two fragments")
        };

        let mut reassembly = Reassembly::new();

        let actual = reassembly.receive_packet(a2.0, a2.1.clone());
        assert_eq!(
            actual,
            ReceivePacketResult::Incomplete(
                Duration::from_secs(30),
                BufId::from_header(&header_a),
                1
            )
        );

        let actual = reassembly.receive_packet(b2.0, b2.1.clone());
        assert_eq!(
            actual,
            ReceivePacketResult::Incomplete(
                Duration::from_secs(30),
                BufId::from_header(&header_b),
                1
            )
        );

        let actual = reassembly.receive_packet(a1.0, a1.1.clone());
        assert_eq!(actual, ReceivePacketResult::Complete(header_a, expected_a),);

        let actual = reassembly.receive_packet(b1.0, b1.1.clone());
        assert_eq!(actual, ReceivePacketResult::Complete(header_b, expected_b),);
    }
}
