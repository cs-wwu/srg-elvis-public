/// Implemntation of RIP v2
/// 
/// See rfc manual entry
/// https://www.rfc-editor.org/rfc/rfc2453
/// 
/// 
/// 
///
const VERSION: u8 = 2;
const AFI_2: u8 = 2;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RipPacket {
    header: RipHeader,
    entries: Vec<RipEntry>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RipHeader {
    pub command: Operation,
    pub version: u8,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RipEntry {
    pub address_family_id: u16,
    pub route_tag: u16,
    pub ip_address: Ipv4Address,
    pub subnet_mask: SubnetMask,
    pub next_hop: Ipv4Address,
    pub metric: u32,
}

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
        Self {
            header,
            entries
        }
    }

    pub fn new_response(entries: Vec<RipEntry>) {
        let header = RipHeader {
            command: Operation::Response,
            version: 2
        };
        RipPacket {
            header,
            entries
        }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();

        out.extend_from_slice(self.header.command.to_be_bytes());
        out.extend_from_slice(self.header.version.to_be_bytes());
        
        // build entries
        for entry in self.entries.iter() {
            out.extend_from_slice(entry.address_family_id.to_be_bytes());
            out.extend_from_slice(entry.route_tag.to_be_bytes());
            out.extend_from_slice((entry.ip_address as u32).to_be_bytes());
            out.extend_from_slice((entry.subnet_mask as u32).to_be_bytes());
            out.extend_from_slice((entry.next_hop as u32).to_be_bytes());
            out.extend_from_slice(entry.metric.to_be_bytes());
        }

        out
    }

    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        const HTS: ParseError = ParseError::HeaderTooShort;

        let command = bytes.next_u8_be().ok_or(HTS)?;
        let command: Operation = match command {
            1 => Operation::Reqest,
            2 => Operation::Response,
            _ => return Err(ParseError::InvalidOperation),
        };

        let version = bytes.next_u8_be().ok_or(HTS)?;

        let mut entries = Vec::new();

        while let Some(address_family_id) = bytes.next_u16_be() {
            let route_tag = bytes.next_u16_be().ok_or(HTS)?;
            let ip_address = bytes.next_u32_be().ok_or(HTS)?.into();
            let subnet_mask = bytes.next_u32_be().ok_or(HTS)?.into();
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

            entries.add(entry);
        }

        if entires.len() > 25 {
            Err(ParseError::HeaderTooLong)
        }

        let header = RipHeader {
            operation,
            version,
        };

        RipPacket {
            header,
            entries,
        }
    }
}

impl RipEntry {
    pub fn new_entry(ip_address: Ipv4Address, metric: u32) -> Self {
        Self {
            address_family_id: AFI_2,
            route_tag: 0,
            ip_address,
            subnet_mask: 0,
            next_hop: 0,
            metric,
        }
    }
}

pub enum ParseError {
    #[error("The RIP header is incomplete")]
    HeaderTooShort,
    #[error("Invalid operation: should be 1 for request, 2 for reply")]
    InvalidOperation,
    #[error("The RIP header has too many entries")]
    HeaderTooLong,
}

mod tests {
    use super::*;

    fn rip_parsing_build_unbuild() {
        let entries = Vec::new();

        for i in (1..16) {
            let ip_address = Ipv4Address::from([192,168,1, i as u8]);
            let metric = i as u32;
            entries.add(RipEntry::new_entry(ip_address, metric));
        }

        let packet = RipPacket::new_request(entries);

        let serialized_packet = packet.build();
        let unserialized_packet = RipPacket::from_bytes(serialized_packet);

        assert_eq!(serialized_packet, packet);
    }    
}