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
    Control, Machine, Protocol, Session, Shutdown,
};

use super::{dns_parsing::{DnsHeader, DnsMessage, DnsMessageType, DnsQuestion, DnsResourceRecord, DnsRTypes}, dns_cache::DnsCache};

use std::any::Any;
use {std::any::TypeId, std::sync::Arc, tokio::sync::Barrier};

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to resource record mappings.
pub struct DnsResolver {
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    cache: DnsCache
}

impl DnsResolver {
    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            cache: DnsCache::new(),
        }
    }

    /// Finds the IP associated with the given domain name. Usuable by external
    /// callers. Specifically intended for use by socket.rs.
    pub async fn get_host_by_name(
        &self,
        name: String,
        machine: Arc<Machine>,
    ) -> Result<Ipv4Address, DnsClientError> {
        match self.cache.get_mapping(&name) {
            // Cache hit
            Ok(rr) => Ok(
                rr.to_ipv4()
            ),
            // Cache miss
            Err(_) => {
                let message = self
                    .create_request(&name)
                    .unwrap()
                    .to_message()
                    .unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                let sockets = machine
                    .protocol::<SocketAPI>()
                    .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))
                    .unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                let mut socket = sockets
                    .new_socket(ProtocolFamily::INET, SocketType::Datagram, machine)
                    .await
                    .unwrap(); // Clean up/handle cleanly unwraps. TODO(HenryEricksonIV)

                // "Connect" the socket to a remote address
                let remote_sock_addr = Endpoint::new(Ipv4Address::DNS_ROOT_AUTH, 53);
                socket.connect(remote_sock_addr).await.unwrap();

                // Send a message
                socket.send(message.to_vec()).unwrap();

                // Receive a message
                let resp = socket.recv_msg().await.unwrap();

                let res_msg = DnsMessage::from_bytes(resp.iter()).unwrap();

                let name_to_add = String::from_utf8(res_msg.question.qname.clone()).unwrap();
                self.cache.add_mapping(name_to_add, res_msg.answers[0].to_owned());


                Ok(self.cache.get_mapping(&name).unwrap().to_ipv4())
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
            1
        );
        let question = DnsQuestion::new(vec_name.clone(), DnsRTypes::A as u16);
        let answers = Vec::from([DnsResourceRecord::new(vec_name, 0, Ipv4Address::new([0, 0, 0, 0]), DnsRTypes::A as u16)]);
        let response_msg = DnsMessage::new(header, question, answers).unwrap();
        Ok(response_msg)
    }
}

#[async_trait::async_trait]
impl Protocol for DnsResolver {
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

impl Default for DnsResolver {
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

    use crate::protocols::dns::dns_cache::DnsCacheError;

    use super::*;

    #[test]
    /// Checks HashMap functionality
    fn add_and_lookup_mapping() {
        // Initialize struct
        let dns: DnsResolver = DnsResolver::new();

        // Create and add mapping
        let domain_name: &str = "Name";
        let name: Vec<u8> = Vec::from("Name");
        let ip: Ipv4Address = Ipv4Address::CURRENT_NETWORK;
        let rr: DnsResourceRecord = DnsResourceRecord::new(name, 1, ip, DnsRTypes::A as u16);
        dns.cache.add_mapping(domain_name.to_string(), rr);

        // Verify that lookup matches what was added
        let check = dns.cache.get_mapping(&domain_name).unwrap();
        assert_eq!(ip, check.to_ipv4());
    }

    #[test]
    // Checks appropriate behaviour on cache miss.
    fn cache_miss() {
        let dns: DnsResolver = DnsResolver::new();

        // Create and do NOT add mapping
        let name: String = String::from("Arbitrary");

        // Verify that lookup returns dns cache miss error.
        let check = dns.cache.get_mapping(&name);
        assert_eq!(Err(DnsCacheError::Cache), check);
    }
}

