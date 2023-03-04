use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        sockets::{
            socket::{ProtocolFamily, SocketAddress, SocketType},
            Sockets,
        },
        user_process::{Application, ApplicationError, UserProcess},
    },
    Id, ProtocolMap,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc::Sender, Barrier};

#[derive(Clone)]
pub struct SocketRecvMessage {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The text of the message to send
    text: &'static str,
    /// The message that was received, if any
    message: Arc<RwLock<Vec<u8>>>,
    /// The address we capture a message on
    _local_ip: Ipv4Address,
    /// The port we capture a message on
    local_port: u16,
    ///// The address we capture a message from
    // remote_ip: Ipv4Address,
    //// The port we capture a message from
    // remote_port: u16,
}

impl SocketRecvMessage {
    pub fn new(
        sockets: Arc<Sockets>,
        text: &'static str,
        _local_ip: Ipv4Address,
        local_port: u16,
        _remote_ip: Ipv4Address,
        _remote_port: u16,
    ) -> Self {
        Self {
            sockets,
            text,
            message: Default::default(),
            _local_ip,
            local_port,
            // remote_ip,
            // remote_port,
        }
    }

    pub fn new_shared(
        sockets: Arc<Sockets>,
        text: &'static str,
        _local_ip: Ipv4Address,
        local_port: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            sockets,
            text,
            _local_ip,
            local_port,
            remote_ip,
            remote_port,
        ))
    }

    pub fn message(&self) -> Vec<u8> {
        self.message.read().unwrap().clone()
    }
}

impl Application for SocketRecvMessage {
    const ID: Id = Id::from_string("Socket Receive");

    fn start(
        self: Arc<Self>,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let listen_socket = self
                .sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();

            // Bind the socket to your local address
            let local_sock_addr =
                SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, self.local_port);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections
            listen_socket.clone().listen(0).unwrap();

            initialized.wait().await;

            // Accept an incoming connection
            let socket = listen_socket.clone().accept().await.unwrap();
            println!("Connection accepted");

            // Send a connection response
            println!("Sending connection response");
            socket.clone().send("ACK").unwrap();

            // Receive a message
            *self.message.write().unwrap() = socket.clone().recv(32).await.unwrap();
            println!(
                "Request Received: {:?}",
                String::from_utf8(self.message.read().unwrap().clone()).unwrap()
            );

            // Send a message
            println!("Sending Response: {:?}", self.text);
            socket.clone().send(self.text).unwrap();

            // Receive another message
            let msg = socket.clone().recv(32).await.unwrap();
            println!("Captured Request: {:?}", String::from_utf8(msg).unwrap());

            shutdown.send(()).await.unwrap();
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
