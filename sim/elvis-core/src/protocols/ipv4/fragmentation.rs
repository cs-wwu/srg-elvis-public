//! Implements section 3.2, An Example Fragmentation Procedure from page 26 of
//! RFC 791 https://www.rfc-editor.org/rfc/rfc791

use super::ipv4_parsing::Ipv4Header;
use crate::{network::Mtu, Message};

type Fragment = (Ipv4Header, Message);

/// The result of packet fragmentation
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
pub enum Fragments {
    /// The packet was fragmented
    Fragmented(Vec<Fragment>),
    /// The packet does not require fragmentation
    DontFragment(Fragment),
    /// The packet required fragmentation but requested not to be fragmented
    Discard,
}

/// Divide the packet into parts that can fit within the MTU of the network
#[allow(unused)]
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

struct Fragmentation {
    fragments: Vec<Fragment>,
    mtu: Mtu,
}

impl Fragmentation {
    fn new(mtu: Mtu) -> Self {
        Self {
            fragments: vec![],
            mtu,
        }
    }

    fn fragment(&mut self, mut header: Ipv4Header, mut body: Message) {
        if header.total_length <= self.mtu {
            self.fragments.push((header, body));
            return;
        }

        // (3) Number of fragment blocks
        let nfb = (self.mtu - header.ihl as u16 * 4) / 8;

        // First fragment
        {
            // (1)
            let mut header = header.clone();

            // (4)
            let body = body.cut(nfb as usize * 8);

            // (5)
            header.flags.set_is_last_fragment(false);
            header.total_length = header.ihl as u16 * 4 + nfb * 8;

            // (6)
            self.fragments.push((header, body))
        }

        // (7)
        // TODO(hardint): Remove options that should not be copied

        // (8)
        // Use whatever is left in `body`

        // (2) Copy old values
        // Note: Most of these just carry forward
        let oihl = header.ihl;

        // (9) Correct the header
        //
        // TODO(hardint): Recompute IHL after removing the not copied options:
        // IHL <- (((OIHL*4)-(length of options not copied))+3)/4
        //
        // Note: Checksum recomputation happens at serialization time
        header.total_length -= nfb * 8 + (oihl - header.ihl) as u16 * 4;
        header.fragment_offset += nfb;

        // (10)
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
        let header = TestHeaderBuilder::with_message_len(LEN)
            .dont_fragment()
            .build();
        assert_eq!(fragment(header, body, MTU), Fragments::Discard);
    }

    #[test]
    fn dont_fragment() {
        const LEN: u16 = 500;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::with_message_len(LEN).build();
        assert_eq!(
            fragment(header.clone(), body.clone(), MTU),
            Fragments::DontFragment((header, body))
        )
    }

    #[test]
    fn fragments_oversize_payload() {
        const LEN: u16 = 2000;
        let body = Message::new(vec![0u8; LEN as usize]);
        let header = TestHeaderBuilder::with_message_len(LEN).build();

        let expected_first = TestHeaderBuilder::new(MTU).more_fragments().build();

        let expected_second = TestHeaderBuilder::with_message_len(LEN - (MTU - HEADER_OCTETS))
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
        let header = TestHeaderBuilder::with_message_len(LEN).build();

        let expected_1 = TestHeaderBuilder::new(MTU_1).more_fragments().build();
        let expected_2 = TestHeaderBuilder::new(MTU_2)
            .more_fragments()
            .fragment_offset(MTU_1 - HEADER_OCTETS)
            .build();
        let expected_3 = TestHeaderBuilder::with_message_len(
            LEN - (MTU_1 - HEADER_OCTETS) - (MTU_2 - HEADER_OCTETS),
        )
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
