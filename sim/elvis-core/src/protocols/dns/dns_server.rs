

use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{Ipv4Address},
        Endpoint,
        Udp,
    },
    Control, Protocol, Session, Shutdown,
    FxDashMap,
};

use super::dns_parsing::{
        DnsHeader,
        DnsQuestion,
        DnsResourceRecord,
        DnsMessageType, DnsMessage,
    };

use {dashmap::mapref::entry::Entry};
use std::sync::Arc;
use tokio::sync::Barrier;


pub const DNS_PORT_NUM: u16 = 53;

pub struct DnsServer {
    /// The DnsServer version of a normal Dns cache to hold all mappings in 
    /// the network.
    name_to_ip: FxDashMap<String, Ipv4Address>
}

impl DnsServer {
    pub fn new(
            name_to_ip: FxDashMap<String, Ipv4Address>,
        ) -> Self {
        Self {
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

    pub fn create_response(
        query_msg: DnsMessage,
        requested_ip: Ipv4Address,
    ) -> Result<DnsMessage, DnsServerError> {
        let header = DnsHeader::new(
            query_msg.header.id,
            DnsMessageType::RESPONSE,
        );
        let question = DnsQuestion::new(query_msg.question.qname);
        let answer = DnsResourceRecord::new(
            query_msg.answer.name,
            query_msg.answer.ttl,
            requested_ip
        );
        let response_msg = DnsMessage::new(header, question, answer).unwrap();
        Ok(response_msg)
    }
}

#[async_trait::async_trait]
impl Protocol for DnsServer {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().unwrap();

        udp.listen(
            self.id(),
            Endpoint::new(Ipv4Address::DNS_AUTH, DNS_PORT_NUM), protocols
        ).unwrap();
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // let client_ip = control.get::<Ipv4Header>().unwrap().source;
        let req_msg = DnsMessage::from_bytes(message.iter()).unwrap();
        match req_msg.get_type() {
            DnsMessageType::QUERY => {
                let name = req_msg.question.query_name().unwrap();
                let address = self.get_mapping(name).unwrap();
                let res_msg = DnsServer::create_response(req_msg, address).unwrap();
                caller.send(DnsMessage::to_message(res_msg).unwrap(), protocols).unwrap();
                Ok(())
            }
            DnsMessageType::RESPONSE => {
                Err(DemuxError::Other)
            }
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsServerError {
    #[error("DNS Authoritative cache lookup error")]
    Cache,
    #[error("DNS Server received response message error")]
    BadRequest,
    #[error("Unspecified DNS Server error")]
    Other,
}
