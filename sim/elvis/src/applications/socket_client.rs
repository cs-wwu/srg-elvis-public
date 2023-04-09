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
    /// Numerical ID
    client_id: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
}

impl SocketClient {
    pub fn new(
        sockets: Arc<Sockets>,
        client_id: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Self {
        Self {
            sockets,
            client_id,
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
        let sockets = self.sockets.clone();
        let remote_ip = self.remote_ip;
        let remote_port = self.remote_port;
        let client_id = self.client_id;

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();

            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            // "Connect" the socket to a remote address
            let remote_sock_addr = SocketAddress::new_v4(remote_ip, remote_port);
            socket.clone().connect(remote_sock_addr).unwrap();

            // Send a connection request
            println!("CLIENT {}: Sending connection request", client_id);
            socket.clone().send("SYN").unwrap();

            // Receive a connection response
            let _ack = socket.clone().recv(32).await.unwrap();
            println!("CLIENT {}: Connection response received", client_id);

            // Send a message
            let req = "Ground Control to Major Tom";
            println!("CLIENT {}: Sending Request: {:?}", client_id, req);
            socket.clone().send(req).unwrap();

            // Receive a message
            let resp = socket.clone().recv(32).await.unwrap();
            println!(
                "CLIENT {}: Response Received: {:?}",
                client_id,
                String::from_utf8(resp).unwrap()
            );

            println!("CLIENT {}: Sending Ackowledgement", client_id);
            socket.clone().send("Ackowledged").unwrap();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
