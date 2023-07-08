

use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{Ipv4Address},
        Endpoint,
        SocketAPI, socket_api::socket::{ProtocolFamily, SocketType, Socket},
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
use std::{sync::Arc, any::TypeId};
use tokio::sync::Barrier;


pub const DNS_PORT_NUM: u16 = 53;

pub struct DnsServer {
    /// The DnsServer version of a normal Dns cache to hold all mappings in 
    /// the network.
    name_to_ip: FxDashMap<String, Ipv4Address>
}

impl DnsServer {
    pub fn new(
        ) -> Self {
        Self {
            name_to_ip: Default::default(),
        }
    }

     /// Adds a new mapping to the name_to_ip cache.
     pub fn add_mapping(&self, name: String, ip: Ipv4Address) {
        self.name_to_ip.insert(name, ip);
    }

    /// Checks local name_to_ip cache for ['Ipv4Address'] given a name.
    pub fn get_mapping(
        table: FxDashMap<String, Ipv4Address>,
        name: String,
    ) -> Result<Ipv4Address, DnsServerError> {
        match table.entry(name) {
            Entry::Occupied(e) => {
                Ok(e.get().clone())
            }
            Entry::Vacant(_) => {
                Err(DnsServerError::Cache)
            }
        }
    }

    async fn respond_to_query(
        table: FxDashMap<String, Ipv4Address>,
        socket: Arc<Socket>,
    ) -> Result<(), DnsServerError> {
        // Receive a message
        println!("SERVER: Waiting for request...");
        let response = socket.recv(4096).await.unwrap();
        let req_msg = DnsMessage::from_bytes(response.iter().cloned()).unwrap();
        println!(
            "SERVER: Request Received"
        );

        let name = req_msg.question.query_name().unwrap();
        let address: Ipv4Address;
        match DnsServer::get_mapping(table, name) {
            Ok(ip) => {
                address = ip;
                
            }
            Err(_) => {
                return Err(DnsServerError::Cache);
            }
        }

        
        // Send a message
        let dns_res_msg = DnsServer::create_response(req_msg, address).unwrap();
        let res_msg = DnsMessage::to_message(dns_res_msg).unwrap();
        println!("SERVER: Sending Response");
        socket.send(res_msg.to_vec()).unwrap();

        // Receive a message (Also example usage of recv_msg)
        // println!("SERVER: Waiting for awkowledgement...");
        let _ack = socket.recv_msg().await.unwrap();
        println!("SERVER: Ackowledgement Received");
        Ok(())
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
        self.add_mapping("testserver.com".to_string(), [123, 45, 67, 89].into());
        self.add_mapping("google.com".to_string(), [123, 45, 67, 90].into());
        self.add_mapping("facebook.com".to_string(), [123, 45, 67, 91].into());
        self.add_mapping("youtube.com".to_string(), [123, 45, 67, 92].into());

        println!("{:?}", self.name_to_ip);

        let sockets = protocols
        .protocol::<SocketAPI>()
        .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = DNS_PORT_NUM;
        let transport = SocketType::Datagram;

        let listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, protocols)
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::CURRENT_NETWORK, local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(0).unwrap();
        println!("\nSERVER: Listening for incoming connections");

        // Wait on ititialization before sending or receiving any message from the network
        initialized.wait().await;

        let mut tasks = Vec::new();
        // Continuously accept incoming connections in a loop, spawning a
        // new tokio task to handle each accepted connection
        loop {
            let table = self.name_to_ip.clone();
            // Accept an incoming connection
            let socket = listen_socket.accept().await.unwrap();
            println!("SERVER: Connection accepted");

            // Spawn a new tokio task for handling communication
            // with the new client
            tasks.push(tokio::spawn(async move {
                let _ = DnsServer::respond_to_query(table, socket).await;
            }));
        }
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
    // #[error("DNS Server received response message error")]
    // BadRequest,
    // #[error("Unspecified DNS Server error")]
    // Other,
}
