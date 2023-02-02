use super::Dns;
use crate::bit;



/// The number of `u32` words in a basic DNS header
const BASE_WORDS: u8 = 6;
/// The number of `u8` bytes in a basic IPv4 header
const BASE_OCTETS: u16 = BASE_WORDS as u16 * 2;

/// A DNS header, as described in RFC1035 p25 s4.1.1
pub(super) struct DnsHeader {
    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    ///  to match up replies to outstanding queries.
    pub id: u16,
    /// the 16 bit wrapper that holds the following fields:
    /// QR, Opcode, AA, TC, RD, RA, Z, RCODE
    pub properties: DNSHeaderProperties,
    /// the number of entries in the question section.
    pub qdcount: u16,
    /// the number of resource records in the answer section.
    pub ancount: u16,
    /// the number of name server resource records in the authority records section.
    pub nscount: u16,
    /// the number of resource records in the additional records section.
    pub arcount: u16,
}

impl DnsHeader {
    /// Parses a header from a byte iterator.
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };


        let id = u16::from_be_bytes([next()?, next()?]);
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The DNS header is incomplete")]
    HeaderTooShort,
}

/// Wrapper struct for holding the QR, Opcode, AA, TC, RD, RA, Z, and RCODE
/// fields of a DNS Header.
pub(super) struct DnsHeaderProperties(u16);

impl DnsHeaderProperties {
    fn get_QR() -> bool {
        self.bit(0)
    }

    fn get_Opcode() -> Self {
        let opcode_range = std::ops::Range {start: 1, end: 5};
        bit_range(opcode_range)
    }

    fn get_AA() -> bool {
        self.bit(5)
    }

    fn get_TC() -> bool {
        self.bit(6)
    }

    fn get_RD() -> bool {
        self.bit(7)
    }

    fn get_RA() -> bool {
        self.bit(8)
    }

    fn get_RCODE() -> Self {
        let rcode_range = std::ops::Range {start: 12, end: 16};
        bit_range(rcode_range)
    }
}

impl BitIndex for DnsHeaderProperties {
    /// DnsHeaderProperties are defined by 2 bytes, 16 bits
    fn bit_length() -> usize {
        2
    }

    fn bit(&self, pos: usize) -> bool {
        self << pos >> self.bit_length()
    }

    fn bit_range(&self, pos: Range<usize>) -> Self {

    }

    fn set_bit(&mut self, pos: usize, val: bool) -> &mut Self {

    }

    fn set_bit_range(&mut self, pos: Range<usize>, val: Self) -> &mut Self {

    }

}

#[cfg(test)]
mod tests {
    use super::*;
}