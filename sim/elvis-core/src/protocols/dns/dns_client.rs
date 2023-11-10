//! An implementation of the Domain Name Structure For Client Machines

use crate::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::Endpoint,
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, SocketType},
        SocketAPI,
    },
    Control, FxDashMap, Machine, Protocol, Session, Shutdown,
};

use super::dns_parsing::{DnsHeader, DnsMessage, DnsMessageType, DnsQuestion, DnsResourceRecord};

use std::any::Any;
use {std::any::TypeId, std::sync::Arc, tokio::sync::Barrier};

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to IP mappings.
pub struct DnsClient {
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    name_to_ip: FxDashMap<String, Ipv4Address>,
}

impl DnsClient {
    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            name_to_ip: Default::default(),
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
    pub fn get_mapping(&self, name: &str) -> Result<Ipv4Address, DnsClientError> {
        match self.name_to_ip.get(name) {
            Some(e) => Ok(*e),
            None => Err(DnsClientError::Cache),
        }
    }

    /// Finds the IP associated with the given domain name. Usuable by external
    /// callers. Specifically intended for use by socket.rs.
    pub async fn get_host_by_name(
        &self,
        name: String,
        machine: Arc<Machine>,
    ) -> Result<Ipv4Address, DnsClientError> {
        match self.get_mapping(&name) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(_ip) => {
                let message = self.create_request(&name).unwrap().to_message().unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                let sockets = machine
                    .protocol::<SocketAPI>()
                    .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))
                    .unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                let mut socket = sockets
                    .new_socket(ProtocolFamily::INET, SocketType::Datagram, machine)
                    .await
                    .unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                // "Connect" the socket to a remote address
                let remote_sock_addr = Endpoint::new(Ipv4Address::DNS_AUTH, 53);
                socket.connect(remote_sock_addr).await.unwrap();

                // Send a message
                socket.send(message.to_vec()).unwrap();

                // Receive a message
                let resp = socket.recv_msg().await.unwrap();

                let res_msg = DnsMessage::from_bytes(resp.iter()).unwrap();

                let name_to_add = String::from_utf8(res_msg.answer.name).unwrap();
                let rdata = res_msg.answer.rdata;
                let ip_to_add = Ipv4Address::new([rdata[0], rdata[1], rdata[2], rdata[3]]);
                self.add_mapping(name_to_add, ip_to_add);

                Ok(self.get_mapping(&name).unwrap())
            }
        }
    }

    /// Properly creates a request using the DnsMessage struct by assembling
    /// necessary sub-structs. Full DnsMessage implementation is WiP
    /// (HenryEricksonIV).
    pub fn create_request(&self, name: &str) -> Result<DnsMessage, DnsClientError> {
        let to_rand: u16 = rand::random::<u16>();
        let vec_name: Vec<u8> = Vec::from(name.to_owned());
        let header = DnsHeader::new(
            // Temporary. Still need to implement unique transaction id's.
            to_rand,
            DnsMessageType::QUERY,
        );
        let question = DnsQuestion::new(vec_name.clone());
        let answer = DnsResourceRecord::new(vec_name, 0, Ipv4Address::new([0, 0, 0, 0]));
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
        _machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

impl Default for DnsClient {
    fn default() -> Self {
        Self::new()
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
        let check = dns.get_mapping(&name);
        assert_eq!(Ok(ip), check);
    }

    #[test]
    // Checks appropriate behaviour on cache miss.
    fn cache_miss() {
        let dns: DnsClient = DnsClient::new();

        // Create and do NOT add mapping
        let name: String = String::from("Arbitrary");

        // Verify that lookup returns dns cache miss error.
        let check = dns.get_mapping(&name);
        assert_eq!(Err(DnsClientError::Cache), check);
    }
}
