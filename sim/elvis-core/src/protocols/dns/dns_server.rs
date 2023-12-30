use crate::{
    machine::Machine,
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

pub const DNS_PORT_NUM: u16 = 53;

pub struct DnsServer {
    /// The DnsServer version of a normal Dns cache to hold all mappings in
    /// the network.
    name_to_ip: FxDashMap<String, Ipv4Address>,
    /// Work around for other issues still being investigated.
    /// The number of open sockets to accept before returning from start.
    num_connections: u16,
}

impl DnsServer {
    pub fn new(num_connections: u16) -> Self {
        Self {
            name_to_ip: Default::default(),
            num_connections,
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
        match table.get(&name) {
            Some(e) => Ok(*e),
            None => Err(DnsServerError::Cache),
        }
    }

    async fn respond_to_query(
        table: FxDashMap<String, Ipv4Address>,
        mut socket: Socket,
    ) -> Result<(), DnsServerError> {
        // Receive a message
        println!("SERVER: Waiting for request...");
        let response = socket.recv(80).await.unwrap();

        let req_msg = DnsMessage::from_bytes(response.iter().cloned()).unwrap();
        println!("SERVER: Request Received");

        let name = req_msg.question.query_name().unwrap();
        let address: Ipv4Address = match DnsServer::get_mapping(table, name) {
            Ok(ip) => ip,
            Err(_) => {
                return Err(DnsServerError::Cache);
            }
        };

        // Send a message
        let dns_res_msg = DnsServer::create_response(req_msg, address).unwrap();
        let res_msg = DnsMessage::to_message(dns_res_msg).unwrap();
        println!("SERVER: Sending Response");
        socket.send(res_msg.to_vec()).unwrap();
        Ok(())
    }

    /// Creates an appropriate response message using DnsMessage and its
    /// related structs. Full DnsMessage implementation is WiP
    /// (HenryEricksonIV).
    pub fn create_response(
        query_msg: DnsMessage,
        requested_ip: Ipv4Address,
    ) -> Result<DnsMessage, DnsServerError> {
        let header = DnsHeader::new(query_msg.header.id, DnsMessageType::RESPONSE);
        let question = DnsQuestion::new(query_msg.question.qname);
        let answer =
            DnsResourceRecord::new(query_msg.answer.name, query_msg.answer.ttl, requested_ip);
        let response_msg = DnsMessage::new(header, question, answer).unwrap();
        Ok(response_msg)
    }
}

impl Protocol for DnsServer {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        // Adds mappings to the dns server cache. This is a stand it method of
        // doing it. TODO (HenryEricksonIV)
        self.add_mapping("testserver.com".to_string(), [123, 45, 67, 15].into());
        self.add_mapping("google.com".to_string(), [123, 45, 67, 60].into());

        let sockets = machine
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = 53;
        let transport = SocketType::Datagram;

        let mut listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, machine)
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::from([0, 0, 0, 0]), local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(10).unwrap();

        // Wait on ititialization before sending or receiving any message from the network
        initialized.wait().await;

        let mut tasks = Vec::new();
        // Continuously accept incoming connections in a loop, spawning a
        // new tokio task to handle each accepted connection
        loop {
            let table = self.name_to_ip.clone();
            // Accept an incoming connection
            let socket = listen_socket.accept().await.unwrap();

            // Spawn a new tokio task for handling communication
            // with the new client
            tasks.push(tokio::spawn(async move {
                DnsServer::respond_to_query(table, socket).await.unwrap();
            }));

            if tasks.len() >= self.num_connections as usize {
                while !tasks.is_empty() {
                    tasks.pop().unwrap().await.unwrap()
                }
                break;
            }
        }
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

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsServerError {
    #[error("DNS Authoritative cache lookup error")]
    Cache,
    // #[error("DNS Server received response message error")]
    // BadRequest,
    #[error("Unspecified DNS Server error")]
    Other,
}
