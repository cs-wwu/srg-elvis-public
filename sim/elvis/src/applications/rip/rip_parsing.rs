use elvis_core::protocols::arp::subnetting::Ipv4Mask;
use elvis_core::protocols::ipv4::Ipv4Address;
use elvis_core::protocols::BytesExt;
use std::fmt::{self, Formatter};

/// Parsing for RIP v2
///
/// See rfc manual entry
/// https://www.rfc-editor.org/rfc/rfc2453
///
const VERSION: u8 = 2;
const AFI_2: u16 = 2;
const INFINITY: u32 = 16;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct RipPacket {
    pub header: RipHeader,
    pub entries: Vec<RipEntry>,
}

impl fmt::Debug for RipPacket {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        for entry in self.entries.iter() {
            println!("{:?}", entry)
        }
        Ok(())
    }
}

impl fmt::Debug for RipEntry {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        print!(
            "ip: {} next hop: {} metric: {}",
            self.ip_address, self.next_hop, self.metric
        );
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RipHeader {
    pub command: Operation,
    pub version: u8,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct RipEntry {
    pub address_family_id: u16,
    pub route_tag: u16,
    pub ip_address: Ipv4Address,
    pub subnet_mask: Ipv4Mask,
    pub next_hop: Ipv4Address,
    pub metric: u32,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Operation {
    Request = 1,
    Response = 2,
}

impl RipPacket {
    pub fn new_request(entries: Vec<RipEntry>) -> Self {
        let header = RipHeader {
            command: Operation::Request,
            version: VERSION,
        };
        Self { header, entries }
    }

    pub fn new_response(entries: Vec<RipEntry>) -> Self {
        let header = RipHeader {
            command: Operation::Response,
            version: VERSION,
        };
        Self { header, entries }
    }

    pub fn new_full_table_request() -> Self {
        let mut entries: Vec<RipEntry> = Vec::new();
        entries.push(RipEntry::new_full_table_request());
        Self::new_request(entries)
    }

    pub fn build(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();

        out.extend_from_slice(&(self.header.command as u8).to_be_bytes());
        out.extend_from_slice(&self.header.version.to_be_bytes());

        // build entries
        for entry in self.entries.iter() {
            out.extend_from_slice(&entry.address_family_id.to_be_bytes());
            out.extend_from_slice(&entry.route_tag.to_be_bytes());
            out.extend_from_slice(&u32::from(entry.ip_address).to_be_bytes());
            out.extend_from_slice(&u32::from(entry.subnet_mask).to_be_bytes());
            out.extend_from_slice(&u32::from(entry.next_hop).to_be_bytes());
            out.extend_from_slice(&entry.metric.to_be_bytes());
        }

        out
    }

    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        const HTS: ParseError = ParseError::HeaderTooShort;

        let command = bytes.next_u8().ok_or(HTS)?;
        let command: Operation = match command {
            1 => Operation::Request,
            2 => Operation::Response,
            _ => return Err(ParseError::InvalidOperation),
        };

        let version = bytes.next_u8().ok_or(HTS)?;

        let mut entries = Vec::new();

        while let Some(address_family_id) = bytes.next_u16_be() {
            let route_tag = bytes.next_u16_be().ok_or(HTS)?;
            let ip_address = bytes.next_u32_be().ok_or(HTS)?.into();
            let subnet_mask = Ipv4Mask::try_from(bytes.next_u32_be().ok_or(HTS)?);

            let subnet_mask = match subnet_mask {
                Ok(mask) => mask,
                Err(_) => return Err(ParseError::Invalid),
            };

            let next_hop = bytes.next_u32_be().ok_or(HTS)?.into();
            let metric = bytes.next_u32_be().ok_or(HTS)?;

            let entry = RipEntry {
                address_family_id,
                route_tag,
                ip_address,
                subnet_mask,
                next_hop,
                metric,
            };

            entries.push(entry);
        }

        if entries.len() > 25 {
            return Err(ParseError::HeaderTooLong);
        }

        let header = RipHeader { command, version };

        Ok(RipPacket { header, entries })
    }
}

impl RipEntry {
    pub fn new_entry(
        ip_address: Ipv4Address,
        next_hop: Ipv4Address,
        subnet_mask: Ipv4Mask,
        metric: u32,
    ) -> Self {
        Self {
            address_family_id: AFI_2,
            route_tag: 0,
            ip_address,
            subnet_mask,
            next_hop,
            metric,
        }
    }

    pub fn new_full_table_request() -> Self {
        Self {
            address_family_id: 0,
            route_tag: 0,
            ip_address: 0.into(),
            subnet_mask: Ipv4Mask::from_bitcount(0),
            next_hop: 0.into(),
            metric: INFINITY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    // #[error("The RIP header is incomplete")]
    HeaderTooShort,
    // #[error("Invalid operation: should be 1 for request, 2 for reply")]
    InvalidOperation,
    // #[error("The RIP header has too many entries")]
    HeaderTooLong,
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rip_parsing_build_unbuild() {
        let mut entries = Vec::new();

        for i in 1..16 {
            let ip_address = Ipv4Address::from([192, 168, 1, i as u8]);
            let metric = i as u32;
            entries.push(RipEntry::new_entry(
                ip_address,
                0.into(),
                Ipv4Mask::from_bitcount(0),
                metric,
            ));
        }

        let packet = RipPacket::new_request(entries);

        let serialized_packet = packet.build();
        let unserialized_packet = RipPacket::from_bytes(serialized_packet.iter().cloned()).unwrap();

        assert_eq!(unserialized_packet, packet);
        println!("new a was: {:?}", unserialized_packet);
    }

    #[test]
    fn rip_parsing_header_too_long() {
        let mut entries = Vec::new();

        for i in 1..27 {
            let ip_address = Ipv4Address::from([192, 168, 1, i as u8]);
            let metric = i as u32;
            entries.push(RipEntry::new_entry(
                ip_address,
                0.into(),
                Ipv4Mask::from_bitcount(0),
                metric,
            ));
        }

        let packet = RipPacket::new_request(entries);

        let serialized_packet = packet.build();
        let unserialized_packet = RipPacket::from_bytes(serialized_packet.iter().cloned())
            .expect_err("packet is too long");

        assert_eq!(unserialized_packet, ParseError::HeaderTooLong);
    }
}
