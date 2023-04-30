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
    ops::Add,
    time::Duration,
};

/// Marks a particular call to [`Reassembly::add_fragment`]. Used to prevent
/// reassembly resources from being cleared if new fragments came in before a
/// timeout expired.
pub type Epoch = u16;

/// Timer lower bound
const TLB: Duration = Duration::from_secs(15);

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
        let fragments = header.fragment_offset + (header.total_length - 1) / 8 + 1;
        let segment = self
            .segments
            .entry(buf_id)
            .or_insert_with(|| Segment::new(fragments));

        match segment.add_fragment(header, body) {
            Some((header, message)) => {
                // (16)
                self.segments.remove(&buf_id).unwrap();
                AddFragmentResult::Complete(header, message)
            }
            None => {
                // (18), (19)
                AddFragmentResult::Incomplete(segment.timer, buf_id, segment.epoch)
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

/// Reassembly resources for a given [`BufId`] datagram identifier.
#[derive(Debug, Clone)]
struct Segment {
    header: Option<Ipv4Header>,
    /// Tracks which fragments have been received. Each bit represent eight
    /// consecutive bytes of the datagram, corresponding to values of the
    /// fragment offset field in an IP header.
    fragment_blocks: BitVec,
    /// Pieces which, taken collectively, will constitute the reassembled
    /// message. May be received out-of-order, hence we do not store an
    /// assembled message directly. The Piece type is set up such that once we
    /// have received all the fragments for a given message, we can just pop all
    /// the messages out of the heap and put them together end-to-end.
    fragments: BinaryHeap<Fragment>,
    /// How long the pending message should be stored before being freed
    timer: Duration,
    /// The current iteration of this data structure. Incremented each time a
    /// fragment arrives.
    epoch: u16,
}

impl Segment {
    /// Creates a new set of reassembly resources for the given segment length
    pub fn new(len: u16) -> Self {
        Self {
            header: None,
            fragment_blocks: BitVec::new(len),
            fragments: Default::default(),
            timer: TLB,
            epoch: 0,
        }
    }

    /// The length of the final segment to be assembled.
    pub fn total_data_length(&self) -> u16 {
        self.fragment_blocks.len
    }

    pub fn add_fragment(
        &mut self,
        header: Ipv4Header,
        body: Message,
    ) -> Option<(Ipv4Header, Message)> {
        // (8)
        self.fragments
            .push(Fragment::new(body, header.fragment_offset));

        // (9)
        self.fragment_blocks.set_range(
            header.fragment_offset,
            header.fragment_offset + ((header.total_length - header.ihl as u16 * 4) + 7) / 8,
        );

        // (10) Ignored. We just find this value when initially constructing the
        // Fragment.

        // (11)
        if header.fragment_offset == 0 {
            self.header = Some(header);
        }

        // (12), (13)
        if self.total_data_length() != 0 && self.fragment_blocks.complete() {
            // (14)
            let mut header = self.header.unwrap();
            header.total_length = self.total_data_length() + header.ihl as u16 * 4;

            // (15)
            let mut message = Message::new(vec![]);
            for piece in self.fragments.drain() {
                message.concatenate(piece.message);
            }

            Some((header, message))
        } else {
            // (17)
            let epoch = self.epoch;
            self.epoch += 1;
            self.timer = self
                .timer
                .max(Duration::from_secs(header.time_to_live as u64));

            None
        }
    }
}

/// A piece of a datagram being assembled. This type is set up to be sorted by
/// the fragment offset, thus allowing easy final reassembly of the collected fragments.
#[derive(Debug, Clone)]
struct Fragment {
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
}

impl PartialEq for Fragment {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

impl Eq for Fragment {}

impl PartialOrd for Fragment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}

impl Ord for Fragment {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset)
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

/// Uniquely identifies the fragments of a particular datagram. See the
/// Identification section for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufId {
    /// The remote IP address
    src: Ipv4Address,
    /// The local IP address
    dst: Ipv4Address,
    /// The transmission protocol used upstream from IP
    protocol: u8,
    /// The identification field of the IP header
    identification: u16,
}

impl BufId {
    /// Gets the segment identifier to a given IP header
    pub fn from_header(header: &Ipv4Header) -> Self {
        Self {
            src: header.source,
            dst: header.destination,
            protocol: header.protocol,
            identification: header.identification,
        }
    }
}

/// A vector of individual bits for space efficiency.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BitVec {
    bits: Vec<u8>,
    len: u16,
}

impl BitVec {
    /// Creates a bit vector of the given length
    fn new(len: u16) -> Self {
        let bytes = (len - 1) / 8 + 1;
        Self {
            bits: vec![0u8; bytes as usize],
            len,
        }
    }

    /// Whether the given bit is set
    fn get(&self, bit: u16) -> bool {
        ((self.bits[bit as usize / 8] >> (bit % 8)) & 1) == 1
    }

    /// Sets the given bit high
    fn set(&mut self, bit: u16) {
        self.bits[bit as usize / 8] |= 1 << (bit % 8);
    }

    /// Sets the range of bits from start (inclusive) to end (exclusive) high.
    fn set_range(&mut self, start: u16, end: u16) {
        // TODO(hardint): Can be made more efficient by setting entire u8 blocks
        // instead of individual bits
        for i in start..end {
            self.set(i)
        }
    }

    /// Checks whether all the bits in the vector have been set high.
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
        let mut bits = BitVec::new(100);
        bits.set_range(10, 50);
        assert!(!bits.get(5));
        assert!(bits.get(10));
        assert!(bits.get(30));
        assert!(bits.get(49));
        assert!(!bits.get(50));
        assert!(!bits.get(75));
    }
}
