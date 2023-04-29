//! Implements section 3.2, An Example Fragmentation Procedure from page 26 of
//! RFC 791 https://www.rfc-editor.org/rfc/rfc791

use super::ipv4_parsing::Ipv4Header;
use crate::{network::Mtu, Message};

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

    fn fragment(&self, mut header: Ipv4Header, mut body: Message) {
        if header.total_length <= self.mtu {
            self.fragments.push((header, body));
            return;
        }

        // (3) Number of fragment blocks
        let mut nfb = (self.mtu - header.ihl as u16 * 4) / 8;

        // First fragment
        {
            // (1)
            let mut header = header.clone();

            // (4)
            let body = body.cut(nfb as usize * 8);

            // (5)
            header.flags.set_may_fragment(true);
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
        let mut oihl = header.ihl;

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

pub fn fragment(header: Ipv4Header, body: Message, mtu: Mtu) -> Fragments {
    if header.total_length <= mtu {
        Fragments::DontFragment((header, body))
    } else if !header.flags.may_fragment() {
        Fragments::Discard
    } else {
        let fragmentation = Fragmentation::new(mtu);
        fragmentation.fragment(header, body);
        Fragments::Fragmented(fragmentation.fragments)
    }
}
