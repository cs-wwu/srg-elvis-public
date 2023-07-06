
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        Endpoint,
        Udp,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        SocketAPI,
    },
    Control, Protocol, Session, Shutdown,
    FxDashMap,
};

use super::dns_parsing::{
        DnsHeader,
        DnsQuestion,
        DnsResourceRecord,
        DnsMessageType,
    };

use {dashmap::mapref::entry::Entry};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;


pub const PORT_NUM: u16 = 53;

pub struct DnsServer {
    /// The Sockets API
    sockets: Arc<Socket>,
    /// The DnsServer version of a normal Dns cache to hold all mappings in 
    /// the network.
    name_to_ip: FxDashMap<String, Ipv4Address>,
}

impl DnsServer {
    pub fn new(
            sockets: Arc<Socket>,
            name_to_ip: FxDashMap<String, Ipv4Address>,
        ) -> Self {
        Self {
            sockets,
            name_to_ip,
        }
    }

     /// Adds a new mapping to the name_to_ip cache.
     pub fn add_mapping(&self, name: String, ip: Ipv4Address) {
        self.name_to_ip.insert(name, ip);
    }

    /// Checks local name_to_ip cache for ['Ipv4Address'] given a name.
    pub fn get_mapping(
        &self,
        name: String,
    ) -> Result<Ipv4Address, DnsServerError> {
        match self.name_to_ip.entry(name) {
            Entry::Occupied(e) => {
                Ok(e.get().clone())
            }
            Entry::Vacant(_) => {
                Err(DnsServerError::Cache)
            }
        }
    }
}

#[async_trait::async_trait]
impl Protocol for DnsServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().unwrap();

        udp.listen(
            self.id(),
            Endpoint::new(Ipv4Address::DNS_AUTH, 53), protocols
        ).unwrap();
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
pub enum DnsServerError {
    #[error("DNS Authoritative cache lookup error")]
    Cache,
    #[error("Unspecified DNS Server error")]
    Other,
}

// impl Default for DnsServer {
//     fn default() -> Self {
//         Self {
//             sockets: Sockets::new(Some(BROADCAST)).shared(),
//         }
//     }
// }
