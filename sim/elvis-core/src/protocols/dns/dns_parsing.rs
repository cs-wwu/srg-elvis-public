// pub mod dns_parsing;

use super::Dns;
// use crate::bit;
use thiserror::Error as ThisError;



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
    /// the 16 bit string that holds the following fields:
    /// QR, Opcode, AA, TC, RD, RA, Z, RCODE
    /// in the format 0 0000 0 0 0 0 000 0000
    pub properties: u16,
    /// the number of entries in the question section.
    pub qdcount: u16,
    /// the number of resource records in the answer section.
    pub ancount: u16,
    /// the number of name server resource records in the authority records section.
    pub nscount: u16,
    /// the number of resource records in the additional records section.
    pub arcount: u16,
}

pub enum DnsMessageType {
    // Indicates the message is a request for information.
    QUERY,
    // Indicates the message is responding to a request.
    RESPONSE,
}

impl DnsHeader {
    /// Parses a header from a byte iterator.
    pub fn from_bytes(mut bytes: impl Iterator<Item = u16>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u16, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };

        let id = next()?;
        let properties = next()?;
        let qdcount = next()?;
        let ancount = next()?;
        let nscount = next()?;
        let arcount = next()?;

        Ok(
            Self {
                id,
                properties,
                qdcount,
                ancount,
                nscount,
                arcount,
            }
        )
    }

    pub fn new(
        message_id: u16,
        message_type: DnsMessageType
    ) -> DnsHeader {
        // Set to nothing for now
        let id = message_id;

        // as binary: 0 0000 0 0 0 0 000 0000
        // Leading bit denotes query or response, remaining fields present for 
        // completeness
        let mut properties = 0x0;
        match message_type {
            DnsMessageType::QUERY       => properties |= 0x0,
            DnsMessageType::RESPONSE    => properties |= 0x8000,
        }

        // Remaining fields of header left as 0x0. Included for completeness.
        let qdcount = 0x0;
        let ancount = 0x0;
        let nscount = 0x0;
        let arcount = 0x0;

        DnsHeader {
            id,
            properties,
            qdcount,
            ancount,
            nscount,
            arcount,
        }
    }

    pub fn build(header: DnsHeader) -> Vec<u16> {
            vec![
                header.id,
                header.properties,
                header.qdcount,
                header.ancount,
                header.nscount,
                header.arcount,
            ]
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The DNS header is incomplete")]
    HeaderTooShort,
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    #[error("The DNS header is invalid")]
    HeaderBadFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_dns_header() {
        let head: DnsHeader = DnsHeader::new(
            1337,
            DnsMessageType::RESPONSE,
        );

        println!("{:?}", head.id);
        println!("{:?}", head.properties);
        println!("{:?}", head.qdcount);
        println!("{:?}", head.ancount);
        println!("{:?}", head.nscount);
        println!("{:?}", head.arcount);

        assert_eq!(head.id, 1337);
        assert_eq!(head.properties, 32768);
        assert_eq!(head.qdcount, 0);
        assert_eq!(head.ancount, 0);
        assert_eq!(head.nscount, 0);
        assert_eq!(head.arcount, 0);
    }

    #[test]
    fn read_dns_header() {
        let head_init: DnsHeader = DnsHeader::new(
            1337,
            DnsMessageType::QUERY,
        );

        let head_as_bytes: Vec<u16> = DnsHeader::build(head_init);

        let head_final: DnsHeader = 
            DnsHeader::from_bytes(head_as_bytes.iter().cloned()).unwrap();

            println!("{:?}", head_final.id);
            println!("{:?}", head_final.properties);
            println!("{:?}", head_final.qdcount);
            println!("{:?}", head_final.ancount);
            println!("{:?}", head_final.nscount);
            println!("{:?}", head_final.arcount);
    
            assert_eq!(head_final.id, 1337);
            assert_eq!(head_final.properties, 0);
            assert_eq!(head_final.qdcount, 0);
            assert_eq!(head_final.ancount, 0);
            assert_eq!(head_final.nscount, 0);
            assert_eq!(head_final.arcount, 0);
    }
}