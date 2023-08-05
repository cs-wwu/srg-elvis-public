use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        Endpoint, SocketAPI,
    },
    Control, FxDashMap, Protocol, Session, Shutdown,
};

use super::dns_parsing::{DnsHeader, DnsMessage, DnsMessageType, DnsQuestion, DnsResourceRecord};

use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

trait DnsRecursiveServer {

    // Given a Vec containing the labels of the name being queried, this fn 
    // will check the cache for a complete match of QNAME, or the next most
    // specific option failing a total match.
    fn find_nearest_ancestor(&self, q_labels: Vec<String>) -> Vec<String> {

    }

}

impl DnsRecursiveServer for DnsServer {
    
}


#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsServerError {
    #[error("DNS Authoritative cache lookup error")]
    Cache,
    #[error("Unspecified DNS Server error")]
    Other,
    #[error("Socket Accept failed")]
    DnsSocket,
}
