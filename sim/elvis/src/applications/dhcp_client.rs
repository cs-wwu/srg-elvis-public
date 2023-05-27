use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        dhcp_parsing::DhcpMessage,
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

// NOTE: THIS IS A TEMPORARY CLIENT
// TO BE DELETED ONCE DHCP HAS BEEN FULLY IMPLEMENTED ON THE CLIENT SIDE
pub struct DhcpClient {
    /// The Sockets API
    sockets: Arc<Sockets>,
    shutdown_bar: Arc<Barrier>,
    chosen: bool,
}

impl DhcpClient {
    pub fn new(sockets: Arc<Sockets>, shutdown_bar: Arc<Barrier>, chosen: bool) -> Self {
        Self {
            sockets,
            shutdown_bar,
            chosen,
        }
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for DhcpClient {
    const ID: Id = Id::from_string("Socket Client");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = self.sockets.clone();
        let chosen = self.chosen;
        let shutdown_bar = self.shutdown_bar.clone();

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols)
                .await
                .unwrap();

            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            socket
                .clone()
                .connect(SocketAddress::new_v4(
                    Ipv4Address::new([255, 255, 255, 255]),
                    67,
                ))
                .unwrap();
            let resp = DhcpMessage::default();
            let resp_msg = DhcpMessage::to_message(resp).unwrap();
            socket.clone().send(resp_msg.to_vec()).unwrap();
            println!("Client Discover sent");
            let msg = socket.clone().recv_msg().await.unwrap();
            let parsed_msg = DhcpMessage::from_bytes(msg.iter()).unwrap();
            println!("Client IP received: {:?}", parsed_msg.your_ip);
            shutdown_bar.wait().await;
            if chosen {
                shutdown.shut_down();
            }
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
