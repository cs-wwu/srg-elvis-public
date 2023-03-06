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

pub struct SocketServer {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The message that was received, if any
    message: RwLock<Vec<u8>>,
    /// The text of the response to send
    text: &'static str,
    /// The port to capture a message on
    local_port: u16,
}

impl SocketServer {
    pub fn new(
        sockets: Arc<Sockets>,
        text: &'static str,
        local_port: u16,
    ) -> Self {
        Self {
            sockets,
            message: Default::default(),
            text,
            local_port,
        }
    }

    pub fn new_shared(
        sockets: Arc<Sockets>,
        text: &'static str,
        local_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            sockets,
            text,
            local_port
        ))
    }

    pub fn message(&self) -> Vec<u8> {
        self.message.read().unwrap().clone()
    }
}

impl Application for SocketServer {
    const ID: Id = Id::from_string("Socket Server");

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

            // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening 
            let local_sock_addr =
                SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, self.local_port);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections
            listen_socket.clone().listen(0).unwrap();

            initialized.wait().await;

            // Accept an incoming connection
            let socket = listen_socket.clone().accept().await.unwrap();
            println!("SERVER: Connection accepted");

            // Send a connection response
            println!("SERVER: Sending connection response");
            socket.clone().send("ACK").unwrap();

            // Receive a message
            *self.message.write().unwrap() = socket.clone().recv(32).await.unwrap();
            println!(
                "SERVER: Request Received: {:?}",
                String::from_utf8(self.message.read().unwrap().clone()).unwrap()
            );

            // Send a message
            println!("SERVER: Sending Response: {:?}", self.text);
            socket.clone().send(self.text).unwrap();

            // Receive another message
            let msg = socket.clone().recv(32).await.unwrap();
            println!("SERVER: Captured Request: {:?}", String::from_utf8(msg).unwrap());

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
