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
    Id, ProtocolMap, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

pub struct SocketServer {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The message that was received, if any
    message: Arc<RwLock<Vec<u8>>>,
    /// The text of the response to send
    text: &'static str,
    /// The port to capture a message on
    local_port: u16,
}

impl SocketServer {
    pub fn new(sockets: Arc<Sockets>, text: &'static str, local_port: u16) -> Self {
        Self {
            sockets,
            message: Default::default(),
            text,
            local_port,
        }
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    pub fn message(&self) -> Vec<u8> {
        self.message.read().unwrap().clone()
    }
}

impl Application for SocketServer {
    const ID: Id = Id::from_string("Socket Server");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Create a new IPv4 Datagram Socket
        let listen_socket = self
            .sockets
            .clone()
            .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
            .unwrap();
        let local_port = self.local_port;
        let text = self.text;
        let message = self.message.clone();

        tokio::spawn(async move {
            // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
            let local_sock_addr = SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, local_port);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections
            listen_socket.clone().listen(0).unwrap();
            println!("SERVER: Listening for incoming connections");

            // Wait on ititialization before receiving any message from the network
            initialized.wait().await;

            // Accept an incoming connection
            let socket = listen_socket.clone().accept().await.unwrap();
            println!("SERVER: Connection accepted");

            // Send a connection response
            println!("SERVER: Sending connection response");
            socket.clone().send("ACK").unwrap();

            // Receive a message
            *message.write().unwrap() = socket.clone().recv(32).await.unwrap();
            println!(
                "SERVER: Request Received: {:?}",
                String::from_utf8(message.read().unwrap().clone()).unwrap()
            );

            // Send a message
            println!("SERVER: Sending Response: {:?}", text);
            socket.clone().send(text).unwrap();

            // Receive another message
            let msg = socket.clone().recv(32).await.unwrap();
            println!(
                "SERVER: Captured Request: {:?}",
                String::from_utf8(msg).unwrap()
            );

            // shutdown.send(()).await.unwrap();
            shutdown.shut_down();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
