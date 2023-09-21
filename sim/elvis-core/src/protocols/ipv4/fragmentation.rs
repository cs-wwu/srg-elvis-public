//! Implements section 3.2, An Example Fragmentation Procedure from page 26 of
//! RFC 791 <https://www.rfc-editor.org/rfc/rfc791>

use super::ipv4_parsing::Ipv4Header;
use crate::{network::Mtu, Message};

/// A piece of a datagram
type Fragment = (Ipv4Header, Message);

/// The result of packet fragmentation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragments {
    /// The packet was fragmented
    Fragmented(Vec<Fragment>),
    /// The packet does not require fragmentation
    DontFragment(Fragment),
    /// The packet required fragmentation but requested not to be fragmented
    Discard,
}

/// Divide the packet into parts that can fit within the MTU of the network
pub fn fragment(header: Ipv4Header, body: Message, mtu: Mtu) -> Fragments {
    if header.total_length <= mtu {
        Fragments::DontFragment((header, body))
    } else if !header.flags.may_fragment() {
        Fragments::Discard
    } else {
        let mut fragmentation = Fragmentation::new(mtu);
        fragmentation.fragment(header, body);
        Fragments::Fragmented(fragmentation.fragments)
    }
}

/// Implements packet fragmentation. Struct introduced for convenience of
/// implementation.
struct Fragmentation {
    /// The fragments of the packet.
    fragments: Vec<Fragment>,
    /// The network maximum transmission unit
    mtu: Mtu,
}

impl Fragmentation {
    /// Creates a new instance
    fn new(mtu: Mtu) -> Self {
        Self {
            fragments: vec![],
            mtu,
        }
    }

    /// Splits a datagram into fragments recursively. One or two fragments are
    /// added to the fragments list for each call.
    fn fragment(&mut self, mut header: Ipv4Header, mut body: Message) {
        if header.total_length <= self.mtu {
            self.fragments.push((header, body));
            return;
        }

        // (3) NFB <- (MTU-IHL*4)/8
        let fragment_blocks = (self.mtu - header.ihl as u16 * 4) / 8;

        // First fragment
        {
            // (1) Copy the original internet header
            let mut header = header;

            // (4) Attach the first NFB*8 data octets
            let body = body.cut(fragment_blocks as usize * 8);

            // (5) Correct the header:
            // MF <- 1;  TL <- (IHL*4)+(NFB*8);
            // Recompute Checksum;
            // NOTE(hardint): Checksum is recomputed when the header is serialized
            header.flags.set_is_last_fragment(false);
            header.total_length = header.ihl as u16 * 4 + fragment_blocks * 8;

            // (6) Submit this fragment to the next step in datagram processing
            self.fragments.push((header, body))
        }

        // (7) Selectively copy the internet header (some options are not
        // copied, see option definitions)
        //
        // TODO(hardint): Remove options that should not be copied

        // (8) Append the remaining data
        // NOTE(hardint): Use whatever is left in body after (4)

        // (2) OIHL <- IHL; OTL <- TL; OFO <- FO; OMF <- MF;
        // NOTE(hardint): Most of these just carry forward
        let oihl = header.ihl;

        // (9) Correct the header:
        // IHL <- (((OIHL*4)-(length of options not copied))+3)/4;
        // TL <- OTL - NFB*8 - (OIHL-IHL)*4);
        // FO <- OFO + NFB;  MF <- OMF;  Recompute Checksum;
        //
        // TODO(hardint): Recompute IHL after removing the not copied options.
        // NOTE(hardint): Checksum is recomputed when the header is serialized.
        header.total_length -= fragment_blocks * 8 + (oihl - header.ihl) as u16 * 4;
        header.fragment_offset += fragment_blocks;

        // (10) Submit this fragment to the fragmentation test
        self.fragment(header, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::ipv4::test_header_builder::TestHeaderBuilder;

    const MTU: Mtu = 1500;
    const HEADER_OCTETS: u16 = 20;

    #[test]
    fn discard() {
        const LEN: u16 = 2000;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::new(LEN).ihl().dont_fragment().build();
        assert_eq!(fragment(header, body, MTU), Fragments::Discard);
    }

    #[test]
    fn dont_fragment() {
        const LEN: u16 = 500;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::new(LEN).ihl().build();
        assert_eq!(
            fragment(header, body.clone(), MTU),
            Fragments::DontFragment((header, body))
        )
    }

    #[test]
    fn fragments_oversize_payload() {
        const LEN: u16 = 2000;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::new(LEN).ihl().build();

        let expected_first = TestHeaderBuilder::new(MTU).more_fragments().build();

        let expected_second = TestHeaderBuilder::new(LEN - (MTU - HEADER_OCTETS))
            .ihl()
            .fragment_offset(MTU - HEADER_OCTETS)
            .build();

        let fragmented = match fragment(header, body, MTU) {
            Fragments::Fragmented(fragmented) => fragmented,
            _ => panic!("Expected fragmented packet"),
        };
        assert_eq!(fragmented.len(), 2);
        assert_eq!(fragmented[0].0, expected_first);
        assert_eq!(fragmented[0].1.len(), (MTU - HEADER_OCTETS) as usize);
        assert_eq!(fragmented[1].0, expected_second);
        assert_eq!(
            fragmented[1].1.len(),
            (LEN - (MTU - HEADER_OCTETS)) as usize
        );
    }

    #[test]
    fn repeated_fragmentation() {
        const MTU_1: Mtu = 1300;
        const MTU_2: Mtu = 500;

        const LEN: u16 = 2000;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::new(LEN).ihl().build();

        let expected_1 = TestHeaderBuilder::new(MTU_1).more_fragments().build();
        let expected_2 = TestHeaderBuilder::new(MTU_2)
            .more_fragments()
            .fragment_offset(MTU_1 - HEADER_OCTETS)
            .build();
        let expected_3 =
            TestHeaderBuilder::new(LEN - (MTU_1 - HEADER_OCTETS) - (MTU_2 - HEADER_OCTETS))
                .ihl()
                .fragment_offset((MTU_1 - HEADER_OCTETS) + (MTU_2 - HEADER_OCTETS))
                .build();

        let fragmented = match fragment(header, body, MTU_1) {
            Fragments::Fragmented(fragmented) => fragmented,
            _ => panic!("Expected fragmented packet"),
        };
        let [actual_1, actual_2] = fragmented.as_slice() else { panic!("Expected two fragments") };

        let fragmented = match fragment(actual_2.0, actual_2.1.clone(), MTU_2) {
            Fragments::Fragmented(fragmented) => fragmented,
            _ => panic!("Expected fragmented packet"),
        };
        let [actual_2, actual_3] = fragmented.as_slice() else { panic!("Expected two fragments") };

        assert_eq!(actual_1.0, expected_1);
        assert_eq!(actual_1.1.len(), (MTU_1 - HEADER_OCTETS) as usize);

        assert_eq!(actual_2.0, expected_2);
        assert_eq!(actual_2.1.len(), (MTU_2 - HEADER_OCTETS) as usize);

        assert_eq!(actual_3.0, expected_3);
        assert_eq!(
            actual_3.1.len(),
            (LEN - (MTU_1 - HEADER_OCTETS) - (MTU_2 - HEADER_OCTETS)) as usize
        )
    }
}
