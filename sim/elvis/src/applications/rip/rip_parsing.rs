/// Implemntation of RIP v2
/// 
/// See rfc manual entry
/// https://www.rfc-editor.org/rfc/rfc2453
/// 
/// 
/// 
///
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RipPacket {
    header: RipHeader,
    entries: Option<Vec<RipEntry>>,
}

pub struct RipHeader {
    pub command: Operation,
    pub version: u8,
}

pub struct RipEntry {
    pub address_family_identifier: u8,
    pub ipv4_address: u32,
    pub _subnet_mask: u32,
    pub _next_hop: u32,
    pub metric: u32,
}

pub enum Operation {
    Request = 1,
    Response = 2,
}

impl RipPacket {
    pub fn new_request() {

    }

    pub fn new_reply() {

    }
}
