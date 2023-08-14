use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        Endpoint, SocketAPI, dns::dns_cache::DnsCacheError,
    },
    Control, Protocol, Session, Shutdown,
};

use std::{any::TypeId, sync::Arc};
// use futures::lock::Mutex;
use slab_tree::Tree;
use tokio::sync::{Barrier, MutexGuard, Mutex};

use super::{dns_parsing::{DnsHeader, DnsMessage, DnsMessageType, DnsQuestion, DnsResourceRecord, DnsRTypes}, domain_name::DomainName, dns_cache::DnsCache, dns_zone_tree::{DnsZoneNode, DnsZoneTree}};


pub const DNS_PORT_NUM: u16 = 53;

pub enum DnsServerType {
    // The DNS authoritative server referring to the root "." of any given domain name.
    ROOT,
    // Any given authoritative server for some given domain or sub-domain.
    AUTH,
    // Any given server which itself contains no authoritative data. [Likely only present for suitably large simulations].
    NOAUTH,
}

/// A struct representing a DNS domain name server intended to hold some number
/// [1..n] of zones of the DNS namespace. Additionally, the name server will
/// store references to the relevant servers if the boundary of a zone is met 
/// for assistance in performing iterative or recursive searches on behalf of
/// a dns_resolver.
pub struct DnsServer {
    /// The DnsServer version of a normal Dns cache to hold mappings.
    cache: DnsCache,
    /// The data structure responsible for holding DNS zone(s).
    zone_tree: DnsZoneTree,
}

impl DnsServer {
    pub fn new() -> Self {
        Self {
            cache: DnsCache::new(),
            zone_tree: DnsZoneTree::new(),
        }
    }

    async fn respond_to_query(
        zone_tree: Arc<Mutex<Tree<DnsZoneNode>>>,
        cache: DnsCache,
        socket: Arc<Socket>,
    ) -> Result<(), DnsServerError> {
        // Receive a message
        let response = socket.recv(80).await.unwrap();

        let req_msg = DnsMessage::from_bytes(response.iter().cloned()).unwrap();

        let name: String = DomainName::from(req_msg.question.qname.to_owned()).into();

        //DEBUG PRINTS
        // SHOWS THAT WE CAN LOOK UP A QNAME IN THE ZONE TREE!!!
        println!("{:?}", name);
        let tree = DnsZoneTree{tree: zone_tree};
        // tree.get_best_zone_match(name.to_owned().into()).await;

        let records: Vec<DnsResourceRecord> = tree.get_best_zone_match(name.to_owned().into()).await.unwrap();

        let cache_lookup: Result<DnsResourceRecord, DnsCacheError> = match DnsCache::get_mapping(&cache, &name) {
            Ok(rr) => Ok(rr),
            Err(_) => Err(DnsCacheError::Cache)
        };

        let rr: DnsResourceRecord = match cache_lookup {
            Ok(rr) => rr,
            Err(_) => {
                records[0].to_owned()
            }
        };
        // if cache_lookup.is_err() {
        //     for r in records {
        //         if r.name_as_labels == DomainName::from(name.to_owned()) {
        //             rr = r;
        //             break
        //         }
        //     }
        // } else {
        //     rr = cache_lookup.unwrap()
        // }

        // Send a message
        let dns_res_msg = DnsServer::create_response(req_msg, rr).unwrap();
        let res_msg = DnsMessage::to_message(dns_res_msg).unwrap();
        socket.send(res_msg.to_vec()).unwrap();
        Ok(())
    }

    /// Creates an appropriate response message using DnsMessage and its
    /// related structs. Full DnsMessage implementation is WiP
    /// (HenryEricksonIV).
    pub fn create_response(
        query_msg: DnsMessage,
        answer_rr: DnsResourceRecord,
    ) -> Result<DnsMessage, DnsServerError> {
        let header = DnsHeader::new(query_msg.header.id, DnsMessageType::RESPONSE);
        let question = DnsQuestion::new(query_msg.question.qname, DnsRTypes::A as u16);
        let answer = answer_rr;
        let response_msg = DnsMessage::new(header, question, answer).unwrap();
        Ok(response_msg)
    }

    /// Continuously accepts new connections and spawns new tokio tasks to 
    /// handle communication with each requester.
    pub async fn accept_loop(
        zone_tree: Arc<Mutex<Tree<DnsZoneNode>>>,
        cache: DnsCache,
        listen_socket: Arc<Socket>
    ) -> Result<(), DnsServerError> {
        loop {
            let table = cache.clone();
            let tree = zone_tree.clone();
            // Accept an incoming connection
            let socket = match listen_socket.accept().await {
                Ok(sock) => sock,
                Err(_) => return Ok(()),
            };
            tokio::spawn(async move {
                DnsServer::respond_to_query(tree, table, socket).await.unwrap();
            });
        }
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

        let root = DnsZoneNode::new(
            ".".to_string(), 
            [
                DnsResourceRecord::new(
                Vec::from("com".as_bytes()),
                1,
                [4, 4, 4, 4].into(),
                DnsRTypes::PTR as u16
                )
            ].to_vec()
        );

        let child_one = DnsZoneNode::new(
            "com".to_string(),
            [
                DnsResourceRecord::new(
                Vec::from("google.com".as_bytes()),
                1,
                [123, 45, 67, 60].into(),
                DnsRTypes::A as u16
                ),
                DnsResourceRecord::new(
                Vec::from("testserver.com".as_bytes()),
                1,
                [123, 45, 67, 15].into(),
                DnsRTypes::A as u16
                )
            ].to_vec()
        );

        let child_two = DnsZoneNode::new(
            "google".to_string(),
            [
                DnsResourceRecord::new(
                Vec::from("google.com".as_bytes()),
                1,
                [123, 45, 67, 60].into(),
                DnsRTypes::A as u16
                )
            ].to_vec()
        );

        let child_three = DnsZoneNode::new(
            "testserver".to_string(),
            [
                DnsResourceRecord::new(
                Vec::from("testserver.com".as_bytes()),
                1,
                [123, 45, 67, 15].into(),
                DnsRTypes::A as u16
                )
            ].to_vec()
        );

        // SLAB TREE

        self.zone_tree.tree_add_root(root).await;
        let root_id = self.zone_tree.tree.lock().await.root_id().unwrap();
        let child_one_id = self.zone_tree.tree_add_child(root_id, child_one).await;
        self.zone_tree.tree_add_child(child_one_id, child_two).await;
        self.zone_tree.tree_add_child(child_one_id, child_three).await;

        // self.tree_print().await;
        
        // Adds mappings to the dns server cache. This is a stand-in method of
        // doing it. TODO (HenryEricksonIV)
        // self.cache.add_mapping(
        //     "testserver.com".to_string(),
        //     DnsResourceRecord::new(
        //         Vec::from("testserver.com".as_bytes()),
        //         1,
        //         [123, 45, 67, 15].into(),
        //         DnsRTypes::A as u16
        //     ));
        // self.cache.add_mapping(
        //     "google.com".to_string(),
        //     DnsResourceRecord::new(
        //         Vec::from("google.com".as_bytes()),
        //         1,
        //         [123, 45, 67, 60].into(),
        //         DnsRTypes::A as u16
        //     ));

        let sockets = protocols
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = 53;
        let transport = SocketType::Datagram;

        let listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, protocols)
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::from([0, 0, 0, 0]), local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(1000).unwrap();

        // Wait on ititialization before sending or receiving any message from the network
        initialized.wait().await;

        // Spawn tokio task to continuously accept incoming 
        // connections in a loop.
        let table = self.cache.clone();
        let tree = self.zone_tree.tree.clone();
        tokio::spawn(async move {
                DnsServer::accept_loop(tree, table, listen_socket).await.unwrap()
            }
        );
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
    #[error("Socket Accept failed")]
    DnsSocket,
}
