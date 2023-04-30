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

pub enum AddFragmentResult {
    /// The added fragment completed the message
    Complete(Ipv4Header, Message),
    /// The added fragment did not complete the message. The caller should set a
    /// timeout for the given duration and call
    /// [`Reassembly::maybe_cull_pending`] with the provided [`BufId`] and
    /// [`Epoch`] after the timeout expires.
    Incomplete(Duration, BufId, Epoch),
}
