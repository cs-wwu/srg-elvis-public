use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        Endpoint, SocketAPI,
    },
    Control, Machine, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

#[derive(Clone)]
pub struct DnsTestServer {
    /// The port to capture a message on
    local_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
}

impl DnsTestServer {
    pub fn new(local_port: u16, transport: SocketType) -> Self {
        Self {
            local_port,
            transport,
        }
    }
}

async fn communicate_with_client(mut socket: Socket) {
    // Receive a message
    let _req = socket.recv(32).await.unwrap();

    // Send a message
    let resp = "Major Tom to Ground Control";
    socket.send(resp).unwrap();

    // Receive a message
    let _ack = socket.recv_msg().await.unwrap();
    println!("SERVER: Ackowledgement Received");
}

pub async fn accept_loop(mut listen_socket: Socket) -> Result<(), DnsTestServerError> {
    loop {
        // Accept an incoming connection
        let socket = match listen_socket.accept().await {
            Ok(sock) => sock,
            Err(_) => return Ok(()),
        };
        
        // Spawn a new tokio task for handling communication
        // with the new client
        tokio::spawn(async move {
            communicate_with_client(socket).await;
        });
    }
}

#[async_trait::async_trait]
impl Protocol for DnsTestServer {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = machine
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = self.local_port;
        let transport = self.transport;

        let mut listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, machine)
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::from([0, 0, 0, 0]), local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(1000).unwrap();

        // Wait on ititialization before sending or receiving any message from the network
        initialized.wait().await;

        tokio::spawn(
            async move {
                accept_loop(listen_socket).await.unwrap()
            }
        );
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
pub enum DnsTestServerError {
    #[error("Unspecified DNS Test Server error")]
    Other,
}
