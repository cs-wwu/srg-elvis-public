//! An implementation of the Domain Name Structure


use elvis_core::{
    // control::{ControlError, Key, Primitive},
    machine::ProtocolMap,
    message::Message,
    protocols::ipv4::Ipv4Address,
    protocol::{DemuxError, StartError},
    protocols::{Udp, Endpoint},
    Control, Protocol, Shutdown, Session,
    FxDashMap,
};

use super::dns_parsing::{
    DnsMessageType, DnsMessage,
};

use std::any::Any;
use {
    dashmap::mapref::entry::Entry,
    std::sync::Arc,
    std::any::TypeId,
    tokio::sync::Barrier,
};

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to IP mappings.
pub struct DnsClient {
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    name_to_ip: FxDashMap<String, Ipv4Address>,

    // Direct reference to Sockets
    // TODO(zachd9757): Replace this with a reference to the Network API once it exists
    // sockets: Sockets,
}

impl DnsClient {

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            name_to_ip: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Adds a new mapping to the name_to_ip cache.
    pub fn add_mapping(&self, name: String, ip: Ipv4Address) {
        self.name_to_ip.insert(name, ip);
    }

    /// Checks local name_to_ip cache for ['Ipv4Address'] given a name.
    pub fn get_mapping(&self, name: String) -> Result<Ipv4Address, DnsClientError> {
        match self.name_to_ip.entry(name) {
            Entry::Occupied(e) => {
                Ok(e.get().clone())
            }
            Entry::Vacant(_e) => {
                Err(DnsClientError::Cache)
            }
        }
    }

    /// Finds the IP associated with the given domain name. Usuable by external
    /// callers. Specifically intended for use by socket.rs.
    pub fn get_host_by_name(
        &self,
        name: String,
        _protocols: ProtocolMap,
    ) -> Result<Ipv4Address, DnsClientError> {
        

        match self.get_mapping(name) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(_ip) => {
                Err(DnsClientError::Other)
            }
        }

        // pub fn create_request(
        //     name: String
        // ) -> Result<DnsMessage, DnsClientError> {
        //     let header = DnsHeader::new(
        //         // Temporary. Still need to implement unique transaction id's.
        //         name.parse::<u16>().unwrap(),   
        //         DnsMessageType::QUERY,
        //     );
        //     let question = DnsQuestion::new(Vec::from(name));
        //     let answer = DnsResourceRecord::new(
        //         Vec::from(name),
        //         0,
        //         Ipv4Address::new([0,0,0,0]),
        //     );
        //     let response_msg = DnsMessage::new(header, question, answer).unwrap();
        //     Ok(response_msg)
        // }
    }
}

#[async_trait::async_trait]
impl Protocol for DnsClient {
    fn id(&self) -> TypeId {
        self.type_id()
    }

    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().unwrap();
        
        udp.listen(
            self.id(),
            Endpoint::new(Ipv4Address::new([0u8, 0, 0, 0]), 53), protocols
        ).unwrap();
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // let client_ip = control.get::<Ipv4Header>().unwrap().source;
        let res_msg = DnsMessage::from_bytes(message.iter()).unwrap();
        match res_msg.get_type() {
            DnsMessageType::QUERY => {
                Err(DemuxError::Other)
            }
            DnsMessageType::RESPONSE => {
                let name_to_add = String::from_utf8(res_msg.answer.name).unwrap();
                let rdata = res_msg.answer.rdata;
                let ip_to_add = Ipv4Address::new([rdata[0], rdata[1], rdata[2], rdata[3]]);
                DnsClient::add_mapping(&self, name_to_add, ip_to_add);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Checks HashMap functionality
    fn add_and_lookup_mapping() {
        // Initialize struct
        let dns: DnsClient = DnsClient::new();

        // Create and add mapping
        let name: String = String::from("Name");
        let ip: Ipv4Address = Ipv4Address::CURRENT_NETWORK;
        dns.add_mapping(name.clone(), ip);

        // Verify that lookup matches what was added
        let check = dns.get_mapping(name);
        assert_eq!(Ok(ip), check);
    }

    #[test]
    // Checks appropriate behaviour on cache miss.
    fn cache_miss() {
        let dns: DnsClient = DnsClient::new();

        // Create and do NOT add mapping
        let name: String = String::from("Arbitrary");

        // Verify that lookup returns dns cache miss error.
        let check = dns.get_mapping(name);
        assert_eq!(Err(DnsClientError::Cache), check);
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsClientError {
    #[error("DNS cache lookup error")]
    Cache,
    #[error("Unspecified DNS error")]
    Other,
}