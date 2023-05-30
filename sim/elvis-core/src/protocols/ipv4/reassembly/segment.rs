//! References page 28 of RFC 791: https://www.rfc-editor.org/rfc/rfc791

use super::{bitvec::BitVec, fragment::Fragment};
use crate::{protocols::ipv4::ipv4_parsing::Ipv4Header, Message};
use std::collections::BinaryHeap;

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
    /// The total length of the reconstructed segment
    total_data_length: u16,
    /// How long the pending message should be stored before being freed
    pub timeout_seconds: u8,
    /// The current iteration of this data structure. Incremented each time a
    /// fragment arrives.
    pub epoch: u16,
}

impl Segment {
    /// Creates a new set of reassembly resources for the given segment length
    pub fn new() -> Self {
        Self {
            header: None,
            fragment_blocks: BitVec::new(),
            fragments: Default::default(),
            timeout_seconds: TLB,
            total_data_length: 0,
            epoch: 0,
        }
    }

    pub fn receive_packet(
        &mut self,
        header: Ipv4Header,
        body: Message,
    ) -> Option<(Ipv4Header, Message)> {
        // (8) put data from fragment into data buffer with BUFID from octet
        // FO*8 to octet (TL-(IHL*4))+FO*8;
        //
        // TODO(hardint): Does not correctly handle fragment blocks that arrive
        // more than once
        self.fragments
            .push(Fragment::new(body, header.fragment_offset));

        // (9) set RCVBT bits from FO to FO+((TL-(IHL*4)+7)/8)
        self.fragment_blocks.set_range(
            header.fragment_offset,
            header.fragment_offset + (header.total_length - header.ihl as u16 * 4 + 7) / 8,
        );

        // (10) IF MF = 0 THEN TDL <- TL-(IHL*4)+(FO*8)
        if header.flags.is_last_fragment() {
            self.total_data_length =
                header.total_length - header.ihl as u16 * 4 + header.fragment_offset * 8;
        }

        // (11) IF FO = 0 THEN put header in header buffer
        if header.fragment_offset == 0 {
            self.header = Some(header);
        }

        // (12) IF TDL # 0
        // (13) AND all RCVBT bits from 0 to (TDL+7)/8 are set
        if self.total_data_length != 0
            && self
                .fragment_blocks
                .complete((self.total_data_length + 7) / 8)
        {
            // (14) THEN TL <- TDL+(IHL*4)
            let mut header = self.header.unwrap();
            header.total_length = self.total_data_length + header.ihl as u16 * 4;
            header.flags.set_is_last_fragment(true);

            // (15) Submit datagram to next step
            let mut message = Message::new(vec![]);
            while let Some(piece) = self.fragments.pop() {
                message.concatenate(piece.into_message());
            }

            Some((header, message))
        } else {
            // (17) TIMER <- MAX(TIMER,TTL)
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
            test_header_builder::TestHeaderBuilder,
        },
    };

    const LEN: u16 = 3000;
    const MTU: Mtu = 500;
    const BASIC_HEADER: Ipv4Header = TestHeaderBuilder::new(LEN).ihl().build();

    #[test]
    fn reassemble_segments_1() {
        let bytes: Vec<_> = (0..LEN).map(|i| i as u8).collect();
        let expected = Message::new(bytes);
        let fragments = match fragment(BASIC_HEADER, expected.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let mut segment = Segment::new();
        for (header, body) in fragments.into_iter().rev() {
            match segment.receive_packet(header, body) {
                Some(actual) => {
                    assert_eq!(actual.0, BASIC_HEADER);
                    assert_eq!(actual.1.len(), expected.len());
                    assert_eq!(actual.1, expected);
                    return;
                }
                None => {}
            }
        }
        panic!("Didn't get a finished message");
    }

    #[test]
    fn reassemble_fragments_2() {
        const LEN: u16 = 1000;
        const MTU: Mtu = 600;

        let bytes_a: Vec<_> = (0..LEN).map(|i| i as u8).collect();
        let expected_a = Message::new(bytes_a);

        let header_a = TestHeaderBuilder::new(LEN).ihl().build();
        let a = match fragment(header_a, expected_a.clone(), MTU) {
            Fragments::Fragmented(fragments) => fragments,
            _ => panic!("Expected fragments"),
        };
        let [a1, a2] = a.as_slice() else { panic!("Expected two fragments") };

        let mut reassembly = Segment::new();

        reassembly.receive_packet(a2.0, a2.1.clone());
        let actual = reassembly.receive_packet(a1.0, a1.1.clone());
        assert_eq!(actual, Some((header_a, expected_a)));
    }
}
