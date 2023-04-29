//! Implements the reassembly procedure from RFC791, section 3.2, page 27: An Example
//! Reassembly Procedure
//! https://www.rfc-editor.org/rfc/rfc791

#![allow(unused)]

use super::{ipv4_parsing::Ipv4Header, Ipv4Address};
use crate::Message;
use rustc_hash::FxHashMap;
use std::{
    cmp::Ordering,
    collections::{hash_map::Entry, BinaryHeap},
    time::Duration,
};

pub type Epoch = u16;

/// Timer lower bound
const TLB: Duration = Duration::from_secs(15);

#[derive(Debug, Default, Clone)]
pub struct Reassembly {
    pending: FxHashMap<BufId, Pending>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufId {
    src: Ipv4Address,
    dst: Ipv4Address,
    protocol: u8,
    identification: u16,
}

impl BufId {
    pub fn from_header(header: &Ipv4Header) -> Self {
        Self {
            src: header.source,
            dst: header.destination,
            protocol: header.protocol,
            identification: header.identification,
        }
    }
}

impl Reassembly {
    pub fn add_fragment(&mut self, header: Ipv4Header, body: Message) -> AddFragmentResult {
        // (1)
        let buf_id = BufId::from_header(&header);
        // (2)
        if header.flags.is_last_fragment() && header.fragment_offset == 0 {
            // (3), (4)
            self.pending.remove(&buf_id);
            // (5)
            return AddFragmentResult::Complete(header, body);
        }

        // (6), (7)
        let fragments = header.fragment_offset + (header.total_length - 1) / 8 + 1;
        let pending = self
            .pending
            .entry(buf_id)
            .or_insert_with(|| Pending::new(fragments));

        // (8)
        pending
            .pieces
            .push(Piece::new(body, header.fragment_offset));

        // (9)
        pending.fragments_received.set_range(
            header.fragment_offset,
            header.fragment_offset + ((header.total_length - header.ihl as u16 * 4) + 7) / 8,
        );

        // (10)
        //
        // TODO(hardint): Is this needed? I feel like we can figure this out
        // when the first segment arrives. Also not sure if TDL is really needed
        // at all. It is probably implicit from the assembled message length.
        if header.flags.is_last_fragment() {
            pending.tdl = header.total_length - header.ihl as u16 * 4 + header.fragment_offset * 8;
        }

        // (11)
        if header.fragment_offset == 0 {
            pending.header = Some(header);
        }

        // (12), (13)
        if pending.tdl != 0 && pending.fragments_received.complete() {
            // (14)
            let mut header = pending.header.unwrap();
            header.total_length = pending.tdl + header.ihl as u16 * 4;

            // (16)
            let pending = self.pending.remove(&buf_id).unwrap();

            // (15)
            let mut message = Message::new(vec![]);
            for piece in pending.pieces.into_iter() {
                message.concatenate(piece.message);
            }
            return AddFragmentResult::Complete(header, message);
        }

        // (17)
        let epoch = pending.epoch;
        pending.epoch += 1;
        pending.timer = pending
            .timer
            .max(Duration::from_secs(header.time_to_live as u64));

        // (18), (19)
        AddFragmentResult::Incomplete(pending.timer, buf_id, epoch)
    }

    pub fn maybe_cull_pending(&mut self, buf_id: BufId, epoch: Epoch) {
        match self.pending.entry(buf_id) {
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

#[derive(Debug, Clone)]
struct Pending {
    header: Option<Ipv4Header>,
    /// Tracks which fragments have been received
    fragments_received: BitSet,
    /// Pieces which, taken collectively, will constitute the reassembled
    /// message. May be received out-of-order, hence we do not store an
    /// assembled message directly. The Piece type is set up such that once we
    /// have received all the fragments for a given message, we can just pop all
    /// the messages out of the heap and put them together end-to-end.
    pieces: BinaryHeap<Piece>,
    /// How long the pending message should be stored before being freed
    timer: Duration,
    /// Total data length
    tdl: u16,
    epoch: u16,
}

impl Pending {
    pub fn new(len: u16) -> Self {
        Self {
            header: None,
            fragments_received: BitSet::new(len),
            pieces: Default::default(),
            timer: TLB,
            tdl: 0,
            epoch: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Piece {
    message: Message,
    offset: u16,
}

impl Piece {
    pub fn new(message: Message, offset: u16) -> Self {
        Self { message, offset }
    }
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

impl Eq for Piece {}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}

impl Ord for Piece {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BitSet {
    bits: Vec<u8>,
    len: u16,
}

impl BitSet {
    fn new(len: u16) -> Self {
        let bytes = (len - 1) / 8 + 1;
        Self {
            bits: vec![0u8; bytes as usize],
            len,
        }
    }

    fn get(&self, bit: u16) -> bool {
        ((self.bits[bit as usize / 8] >> (bit % 8)) & 1) == 1
    }

    fn set(&mut self, bit: u16) {
        self.bits[bit as usize / 8] |= 1 << (bit % 8);
    }

    fn set_range(&mut self, start: u16, end: u16) {
        // TODO(hardint): Can be made more efficient by setting entire u8 blocks
        // instead of individual bits
        for i in start..end {
            self.set(i)
        }
    }

    fn complete(&self) -> bool {
        // TODO(hardint): Can be made more efficient by checking entire u8 blocks
        // instead of individual bits
        (0..self.len).all(|i| self.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set() {
        let mut bits = BitSet::new(100);
        bits.set_range(10, 50);
        assert!(!bits.get(5));
        assert!(bits.get(10));
        assert!(bits.get(30));
        assert!(bits.get(49));
        assert!(!bits.get(50));
        assert!(!bits.get(75));
    }
}
