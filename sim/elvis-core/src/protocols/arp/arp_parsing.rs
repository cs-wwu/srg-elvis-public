//! This module contains things which can be used to create ARP packets
//! and decompose them into IPs and MACs.
//! Currently, ARP packets in ELVIS are modelled after
//! IPv4 over Ethernet ARP packets.
//!
//! <https://en.wikipedia.org/wiki/Address_Resolution_Protocol#Packet_structure>

use crate::{network::Mac, protocols::ipv4::Ipv4Address, protocols::utility::BytesExt};
use thiserror::Error as ThisError;

// This stuff is useless in ELVIS, but I decided to include it
// because this is what real ARP packets have
const HTYPE: u16 = 1;
const PTYPE: u16 = 0x0800;
const HLEN: u8 = 6;
const PLEN: u8 = 4;

/// A struct representing an ARP packet.
/// While this has all the fields of a real ARP packet,
/// it is currently intended only for Ipv4 over Ethernet (which is what ELVIS networks use).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ArpPacket {
    /// The network link protocol type.
    pub htype: u16,
    /// The internetwork protocol for which the ARP request is intended.
    pub ptype: u16,
    /// The length (in octets) of a hardware address. Ethernet address length is 6.
    pub hlen: u8,
    /// Length (in octets) of internetwork addresses. Ipv4 address length is 4.
    pub plen: u8,
    /// Specifies the operation that the sender is performing: 1 for request, 2 for reply.
    pub oper: Operation,
    /// The MAC address of the sender.
    pub sender_mac: Mac,
    /// The Ipv4 address of the sender.
    pub sender_ip: Ipv4Address,
    /// The MAC address of the target.
    pub target_mac: Mac,
    /// The Ipv4 addrses of the target.
    pub target_ip: Ipv4Address,
}

impl ArpPacket {
    /// The size of an ARP packet in bytes (28).
    pub const SIZE: usize = 28;

    /// Initializes an Ipv4 over Ethernet Arp request.
    pub fn new_request(
        sender_mac: Mac,
        sender_ip: Ipv4Address,
        target_ip: Ipv4Address,
    ) -> ArpPacket {
        ArpPacket {
            htype: HTYPE,
            ptype: PTYPE,
            hlen: HLEN,
            plen: PLEN,
            oper: Operation::Request,
            sender_mac,
            sender_ip,
            target_mac: 69,
            target_ip,
        }
    }

    /// Initializes an Ipv4 over Ethernet Arp reply.
    pub fn new_reply(
        sender_mac: Mac,
        sender_ip: Ipv4Address,
        target_mac: Mac,
        target_ip: Ipv4Address,
    ) -> ArpPacket {
        ArpPacket {
            htype: HTYPE,
            ptype: PTYPE,
            hlen: HLEN,
            plen: PLEN,
            oper: Operation::Reply,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }

    /// Creates a serialized ARP packet from the configuration provided.
    pub fn build(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(Self::SIZE);
        // add htype, ptype, hlen, plen (useless)
        out.extend_from_slice(&self.htype.to_be_bytes());
        out.extend_from_slice(&self.ptype.to_be_bytes());
        out.extend_from_slice(&self.hlen.to_be_bytes());
        out.extend_from_slice(&self.plen.to_be_bytes());

        let operation: u16 = self.oper as u16;
        out.extend_from_slice(&operation.to_be_bytes());

        // MAC addresses are 6 bytes long
        out.extend_from_slice(&self.sender_mac.to_be_bytes()[2..8]);
        out.extend_from_slice(&self.sender_ip.to_bytes());
        out.extend_from_slice(&self.target_mac.to_be_bytes()[2..8]);
        out.extend_from_slice(&self.target_ip.to_bytes());
        out
    }

    /// Parses an ARP packet from a byte iterator.
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        const HTS: ParseError = ParseError::HeaderTooShort;

        // get htype, ptype, hlen, plen (useless)
        let htype = bytes.next_u16_be().ok_or(HTS)?;
        let ptype = bytes.next_u16_be().ok_or(HTS)?;
        let hlen = bytes.next_u8().ok_or(HTS)?;
        let plen = bytes.next_u8().ok_or(HTS)?;

        // get operation
        let oper = bytes.next_u16_be().ok_or(HTS)?;
        let oper: Operation = match oper {
            1 => Operation::Request,
            2 => Operation::Reply,
            _ => return Err(ParseError::InvalidOperation),
        };

        // get MACs and IPs
        let sender_mac = bytes.next_u48_be().ok_or(HTS)?;
        let sender_ip: Ipv4Address = bytes.next_ipv4addr().ok_or(HTS)?;
        let target_mac = bytes.next_u48_be().ok_or(HTS)?;
        let target_ip: Ipv4Address = bytes.next_ipv4addr().ok_or(HTS)?;
        Ok(Self {
            htype,
            ptype,
            hlen,
            plen,
            oper,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }
}
#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum PacketBuildError {}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The ARP header is incomplete")]
    HeaderTooShort,
    #[error("Invalid operation: should be 1 for request, 2 for reply")]
    InvalidOperation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_parsing_build_unbuild() -> anyhow::Result<()> {
        let old_a = ArpPacket::new_reply(
            1337,                               // sender_mac
            Ipv4Address::new([127, 0, 0, 1]),   // sender_ip
            70368744177664,                     // target_mac
            Ipv4Address::new([10, 11, 12, 13]), // target_ip
        );

        let a_bytes = old_a.build();
        let new_a = ArpPacket::from_bytes(a_bytes.iter().cloned())?;

        assert_eq!(old_a, new_a);
        println!("new a was: {:?}", new_a);

        Ok(())
    }

    #[test]
    fn arp_parsing_too_short() {
        let short_packet: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
        ArpPacket::from_bytes(short_packet.iter().cloned())
            .expect_err("packet was too short; should not have been built");
    }
}

/// Represents a request or reply operation of an ARP packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Request = 1,
    Reply = 2,
}
