//! Implements the reassembly procedure from RFC791, section 3.2, page 27: An Example
//! Reassembly Procedure
//! https://www.rfc-editor.org/rfc/rfc791

#![allow(unused)]

mod bitvec;
mod buf_id;
mod fragment;
mod segment;

use self::{buf_id::BufId, segment::Epoch};
use super::{ipv4_parsing::Ipv4Header, Ipv4Address};
use crate::Message;
use rustc_hash::FxHashMap;
use segment::Segment;
use std::{
    cmp::Ordering,
    collections::{hash_map::Entry, BinaryHeap},
    ops::Add,
    time::Duration,
};

/// Manages the reassembly of fragmented IP packets.
#[derive(Debug, Default, Clone)]
pub struct Reassembly {
    /// Fragmented IP packets that are still waiting on fragments to become
    /// complete.
    segments: FxHashMap<BufId, Segment>,
}

impl Reassembly {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_fragment(&mut self, header: Ipv4Header, body: Message) -> AddFragmentResult {
        // (1)
        let buf_id = BufId::from_header(&header);
        // (2)
        if header.flags.is_last_fragment() && header.fragment_offset == 0 {
            // (3), (4)
            self.segments.remove(&buf_id);
            // (5)
            return AddFragmentResult::Complete(header, body);
        }

        // (6), (7)
        let segment = self
            .segments
            .entry(buf_id)
            .or_insert_with(|| Segment::from_header(&header));

        match segment.add_fragment(header, body) {
            Some((header, message)) => {
                // (16)
                self.segments.remove(&buf_id).unwrap();
                AddFragmentResult::Complete(header, message)
            }
            None => {
                // (18), (19)
                AddFragmentResult::Incomplete(
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AddFragmentResult {
    /// The added fragment completed the message
    Complete(Ipv4Header, Message),
    /// The added fragment did not complete the message. The caller should set a
    /// timeout for the given duration and call
    /// [`Reassembly::maybe_cull_pending`] with the provided [`BufId`] and
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
            ipv4_parsing::{ControlFlags, TypeOfService},
            reassembly, Ipv4Address,
        },
    };

    const LEN: u16 = 1000;
    const MTU: Mtu = 600;
    const BASIC_HEADER: Ipv4Header = Ipv4Header {
        total_length: LEN + 20,
        flags: ControlFlags::DEFAULT,
        fragment_offset: 0,
        ihl: 5,
        type_of_service: TypeOfService::DEFAULT,
        identification: 1337,
        time_to_live: 30,
        protocol: 17,
        checksum: 0,
        source: Ipv4Address::CURRENT_NETWORK,
        destination: Ipv4Address::CURRENT_NETWORK,
    };

    #[test]
    fn reassemble_segments() {
        let bytes_a: Vec<_> = (0..LEN).map(|i| i as u8).collect();
        let expected_a = Message::new(bytes_a);

        let bytes_b: Vec<_> = (0..LEN).map(|i| (i as u8).wrapping_add(5)).collect();
        let expected_b = Message::new(bytes_b);

        let a = match fragment(BASIC_HEADER, expected_a.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let [a1, a2] = a.as_slice() else { panic!("Expected two fragments") };

        let mut second_header = BASIC_HEADER;
        second_header.identification = 420;
        let b = match fragment(second_header, expected_b.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let [b1, b2] = b.as_slice() else { panic!("Expected two fragments") };

        let mut reassembly = Reassembly::new();
        assert_eq!(
            reassembly.add_fragment(a2.0, a2.1.clone()),
            AddFragmentResult::Incomplete(
                Duration::from_secs(30),
                BufId::from_header(&BASIC_HEADER),
                1
            )
        );
        assert_eq!(
            reassembly.add_fragment(b2.0, b2.1.clone()),
            AddFragmentResult::Incomplete(
                Duration::from_secs(30),
                BufId::from_header(&second_header),
                1
            )
        );
        assert_eq!(
            reassembly.add_fragment(a1.0, a1.1.clone()),
            AddFragmentResult::Complete(BASIC_HEADER, expected_a),
        );
        assert_eq!(
            reassembly.add_fragment(b1.0, b1.1.clone()),
            AddFragmentResult::Complete(second_header, expected_b),
        );
    }
}

fn bytes_to_fragments(bytes: u16) -> u16 {
    (bytes - 1) / 8 + 1
}
