use super::{bitvec::BitVec, fragment::Fragment};
use crate::{protocols::ipv4::ipv4_parsing::Ipv4Header, Message};
use std::{collections::BinaryHeap, time::Duration};

/// Timer lower bound
const TLB: u8 = 15;

/// Marks a particular call to [`Reassembly::add_fragment`]. Used to prevent
/// reassembly resources from being cleared if new fragments came in before a
/// timeout expired.
pub type Epoch = u16;

/// Reassembly resources for a given [`BufId`] datagram identifier.
#[derive(Debug, Clone)]
pub struct Segment {
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
    pub timeout_seconds: u8,
    /// The current iteration of this data structure. Incremented each time a
    /// fragment arrives.
    pub epoch: u16,
}

impl Segment {
    /// Creates a new set of reassembly resources for the given segment length
    fn new(fragment_blocks: u16) -> Self {
        Self {
            header: None,
            fragment_blocks: BitVec::new(fragment_blocks),
            fragments: Default::default(),
            timeout_seconds: TLB,
            epoch: 0,
        }
    }

    pub fn from_total_length(bytes: u16) -> Self {
        Self::new(bytes_to_fragments(bytes))
    }

    pub fn from_header(header: &Ipv4Header) -> Self {
        Self::new(header.fragment_offset + bytes_to_fragments(header.total_length))
    }

    /// The length of the final segment to be assembled.
    pub fn total_data_length(&self) -> u16 {
        self.fragment_blocks.len()
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
            header.fragment_offset + (header.total_length - header.ihl as u16 * 4 + 7) / 8,
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
                message.concatenate(piece.into_message());
            }

            Some((header, message))
        } else {
            // (17)
            let epoch = self.epoch;
            self.epoch += 1;
            self.timeout_seconds = self.timeout_seconds.max(header.time_to_live);

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::Mtu,
        protocols::ipv4::{
            fragmentation::{fragment, Fragments},
            ipv4_parsing::{ControlFlags, TypeOfService},
            Ipv4Address,
        },
    };

    const LEN: u16 = 3000;
    const MTU: Mtu = 500;
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
    fn reassemble_segment_in_order() {
        let bytes: Vec<_> = (0..LEN).map(|i| i as u8).collect();
        let expected = Message::new(bytes);
        let fragments = match fragment(BASIC_HEADER, expected.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let mut segment = Segment::from_total_length(LEN);
        for (header, body) in fragments {
            match segment.add_fragment(header, body) {
                Some(actual) => {
                    assert_eq!(actual.0, BASIC_HEADER);
                    assert_eq!(actual.1, expected);
                }
                None => {}
            }
        }
        panic!("Didn't get a finished message");
    }
}

fn bytes_to_fragments(bytes: u16) -> u16 {
    (bytes - 1) / 8 + 1
}
