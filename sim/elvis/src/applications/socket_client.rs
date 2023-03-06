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
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

pub struct SocketClient {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The text of the message to send
    text: &'static str,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
}

impl SocketClient {
    pub fn new(
        sockets: Arc<Sockets>,
        text: &'static str,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Self {
        Self {
            sockets,
            text,
            remote_ip,
            remote_port,
        }
    }

    pub fn new_shared(
        sockets: Arc<Sockets>,
        text: &'static str,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            sockets,
            text,
            remote_ip,
            remote_port,
        ))
    }
}

impl Application for SocketClient {
    const ID: Id = Id::from_string("Socket Client");

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let socket = self
                .sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();

            // Bind the socket to your local address
            // let local_sock_addr = SocketAddress::new_v4(self.local_ip, self.local_port);
            // socket.clone().bind(local_sock_addr).unwrap();

            // "Connect" the socket to a remote address
            let remote_sock_addr = SocketAddress::new_v4(self.remote_ip, self.remote_port);
            socket.clone().connect(remote_sock_addr).unwrap();

            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            // Send a connection request
            println!("CLIENT: Sending connection request");
            socket.clone().send("SYN").unwrap();

            // Receive a connection response
            let _ack = socket.clone().recv(32).await.unwrap();
            println!("CLIENT: Connection response received");

            // Send a message
            println!("CLIENT: Sending Request: {:?}", self.text);
            socket.clone().send(self.text).unwrap();

            // Receive a message
            let msg = socket.clone().recv(32).await.unwrap();
            println!("CLIENT: Response Received: {:?}", String::from_utf8(msg).unwrap());

            // Send another message
            println!("CLIENT: Sending Request: \"Shutdown\"");
            socket.clone().send("Shutdown").unwrap();
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
