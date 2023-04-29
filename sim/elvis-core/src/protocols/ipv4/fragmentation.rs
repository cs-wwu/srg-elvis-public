//! Implements section 3.2, An Example Fragmentation Procedure from page 26 of
//! RFC 791 https://www.rfc-editor.org/rfc/rfc791

use super::ipv4_parsing::Ipv4Header;
use crate::{network::Mtu, Message};

/// The result of packet fragmentation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragments {
    /// The packet was fragmented
    Fragmented(Vec<(Ipv4Header, Message)>),
    /// The packet does not require fragmentation
    DontFragment(Ipv4Header, Message),
    /// The packet required fragmentation but requested not to be fragmented
    Discard,
}

pub fn fragment(mut header: Ipv4Header, mut body: Message, mtu: Mtu) -> Fragments {
    if header.total_length <= mtu {
        return Fragments::DontFragment(header, body);
    }

    if !header.flags.may_fragment() {
        return Fragments::Discard;
    }

    // (2) Copy old values
    let mut oihl = header.ihl;
    let mut otl = header.total_length;
    let mut ofo = header.fragment_offset;
    let mut omf = header.flags.may_fragment();

    // (3) Number of fragment blocks
    let mut nfb = (mtu - header.ihl as u16 * 4) / 8;

    let mut fragments = {
        // (1)
        let mut first_header = header.clone();

        // (4)
        let first_body = body.cut(nfb as usize * 8);

        // (5) Correct the header
        first_header.flags.set_may_fragment(true);
        first_header.total_length = first_header.ihl as u16 * 4 + nfb as u16 * 8;
        vec![(first_header, first_body)]
    };

    while header.total_length > mtu {}

    Fragments::Fragmented(fragments)
}
