//! This module contains things which can be used to create ARP packets
//! and decompose them into IPs and MACs.
//! Currently, ARP packets in ELVIS are modelled after
//! IPv4 over Ethernet ARP packets.
//!
//! https://en.wikipedia.org/wiki/Address_Resolution_Protocol#Packet_structure

use crate::{
    network::Mac,
    protocols::{ipv4::Ipv4Address, Ipv4},
    Id,
};
use thiserror::Error as ThisError;

// This stuff is useless in ELVIS, but I decided to include it
// because this is what real ARP packets have
const HTYPE: u16 = 1;
const PTYPE: Id = Ipv4::ID;
const HLEN: u8 = 6;
const PLEN: u8 = 4;

/// A struct representing an ARP packet.
#[derive(Debug, PartialEq, Eq, Copy, Hash, Clone)]
pub struct ArpPacket {
    /// Should be 1 for a request, 2 for a reply
    pub is_request: bool,
    pub sender_mac: Mac,
    pub sender_ip: Ipv4Address,
    pub target_mac: Mac,
    pub target_ip: Ipv4Address,
}

impl ArpPacket {
    /// Creates a serialized ARP packet from the configuration provided.
    pub fn build(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        // These 4 lines are useless for ELVIS
        // but I included them anyway so this is like a real ARP packet
        out.extend_from_slice(&HTYPE.to_be_bytes());
        out.extend_from_slice(&(PTYPE.into_inner() as u16).to_be_bytes());
        out.extend_from_slice(&HLEN.to_be_bytes());
        out.extend_from_slice(&PLEN.to_be_bytes());

        let operation: u16 = match self.is_request {
            true => 1,
            false => 2,
        };
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
        // Skip HTYPE, PTYPE, HLEN, PLEN (bruh)
        bytes.nth(5);
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };

        let operation = u16::from_be_bytes([next()?, next()?]);
        let is_request = match operation {
            1 => true,
            2 => false,
            _ => return Err(ParseError::InvalidOperation),
        };
        let sender_mac =
            u64::from_be_bytes([0, 0, next()?, next()?, next()?, next()?, next()?, next()?]);
        let sender_ip: Ipv4Address =
            u32::from_be_bytes([next()?, next()?, next()?, next()?]).into();
        let target_mac =
            u64::from_be_bytes([0, 0, next()?, next()?, next()?, next()?, next()?, next()?]);
        let target_ip: Ipv4Address =
            u32::from_be_bytes([next()?, next()?, next()?, next()?]).into();
        Ok(Self {
            is_request,
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
        let old_a = ArpPacket {
            is_request: true,
            sender_mac: 1337,
            sender_ip: Ipv4Address::new([127, 0, 0, 1]),
            target_mac: 70368744177664,
            target_ip: Ipv4Address::new([10, 11, 12, 13]),
        };

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
