//! An implementation of the Domain Name Structure


use crate::{
    // control::{ControlError, Key, Primitive},
    machine::ProtocolMap,
    message::Message,
    protocols::{ipv4::Ipv4Address, Endpoints, SocketAPI, socket_api::socket::{ProtocolFamily, SocketType}},
    protocol::{DemuxError, StartError},
    protocols::{Udp, Endpoint},
    Control, Protocol, Shutdown, Session,
    FxDashMap,
};

use super::dns_parsing::{
    DnsMessageType,
    DnsMessage,
    DnsHeader,
    DnsQuestion,
    DnsResourceRecord,
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
    // protocols: ProtocolMap,
}

impl DnsClient {

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            name_to_ip: Default::default(),
            // protocols,
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn _shared(self) -> Arc<Self> {
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
    pub async fn get_host_by_name(
        &self,
        name: String,
        protocols: ProtocolMap,
    ) -> Result<Ipv4Address, DnsClientError> {
        

        match self.get_mapping(name.clone()) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(_ip) => {

                let message = DnsMessage::to_message(DnsClient::create_request(name.clone()).unwrap()).unwrap();

                let sockets = protocols
                    .protocol::<SocketAPI>()
                    .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>())).unwrap();
    
                let socket = sockets
                    .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols)
                    .await
                    .unwrap();

                // "Connect" the socket to a remote address
                let remote_sock_addr = Endpoint::new(Ipv4Address::DNS_AUTH, 53);
                socket.connect(remote_sock_addr).await.unwrap();
                println!("CLIENT: Connected");

                // Send a message
                println!("CLIENT: Sending Request:");
                socket.send(message.to_vec()).unwrap();

                // Receive a message
                let resp = socket.recv(message.len()).await.unwrap();
                println!(
                    "CLIENT: Response Received"
                );

                let res_msg = DnsMessage::from_bytes(resp.iter().cloned()).unwrap();

                let name_to_add = String::from_utf8(res_msg.answer.name).unwrap();
                let rdata = res_msg.answer.rdata;
                let ip_to_add = Ipv4Address::new([rdata[0], rdata[1], rdata[2], rdata[3]]);
                DnsClient::add_mapping(&self, name_to_add, ip_to_add);
                        
                Ok(self.get_mapping(name.clone()).unwrap())
            }
        }

    }

    pub fn create_request(
        name: String
    ) -> Result<DnsMessage, DnsClientError> {
        let vec_name: Vec<u8> = Vec::from(name.clone());
        let header = DnsHeader::new(
            // Temporary. Still need to implement unique transaction id's.
            name.parse::<u16>().unwrap(),   
            DnsMessageType::QUERY,
        );
        let question = DnsQuestion::new(vec_name.clone());
        let answer = DnsResourceRecord::new(
            vec_name.clone(),
            0,
            Ipv4Address::new([0,0,0,0]),
        );
        let response_msg = DnsMessage::new(header, question, answer).unwrap();
        Ok(response_msg)
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
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsClientError {
    #[error("DNS cache lookup error")]
    Cache,
    #[error("Unspecified DNS error")]
    Other,
}

#[cfg(test)]
mod tests {
    use crate::{new_machine};

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
