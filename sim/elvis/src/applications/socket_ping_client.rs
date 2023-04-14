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

pub struct SocketPingClient {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// Numerical ID
    client_id: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
}

impl SocketPingClient {
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

impl Application for SocketPingClient {
    const ID: Id = Id::from_string("Socket Ping Client");

    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = self.sockets.clone();
        let remote_ip = self.remote_ip;
        let remote_port = self.remote_port;
        let client_id = self.client_id;

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols)
                .await
                .unwrap();

            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            // "Connect" the socket to a remote address
            let remote_sock_addr = SocketAddress::new_v4(remote_ip, remote_port);
            socket.clone().connect(remote_sock_addr).unwrap();

            // Send a message
            socket.clone().send(vec![255]).unwrap();

            loop {
                // Receive a message
                let mut ttl: u8 = *socket.clone().recv(8).await.unwrap().first().unwrap();
        
                // Send a message
                ttl -= 1;

                socket.clone().send(vec![ttl]).unwrap();
                if ttl <= 1 {
                    break;
                }
            }
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
