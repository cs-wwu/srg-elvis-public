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
use std::sync::Arc;
use tokio::sync::Barrier;

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

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for SocketClient {
    const ID: Id = Id::from_string("Socket Client");

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Create a new IPv4 Datagram Socket
        let sockets = self.sockets.clone();
        let remote_ip = self.remote_ip;
        let remote_port = self.remote_port;
        let text = self.text;
        // "Connect" the socket to a remote address
        let remote_sock_addr = SocketAddress::new_v4(remote_ip, remote_port);

        tokio::spawn(async move {
            // Wait on initialization before sending any message across the network
            initialized.wait().await;
            let socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();
            socket.clone().connect(remote_sock_addr).unwrap();

            // Send a connection request
            println!("CLIENT: Sending connection request");
            socket.clone().send("SYN").unwrap();

            // Receive a connection response
            let _ack = socket.clone().recv(32).await.unwrap();
            println!("CLIENT: Connection response received");

            // Send a message
            println!("CLIENT: Sending Request: {:?}", text);
            socket.clone().send(text).unwrap();

            // Receive a message
            let msg = socket.clone().recv(32).await.unwrap();
            println!(
                "CLIENT: Response Received: {:?}",
                String::from_utf8(msg).unwrap()
            );

            // Send another message
            println!("CLIENT: Sending Request: \"Shutdown\"");
            socket.clone().send("Shutdown").unwrap();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
