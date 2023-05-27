use crate::applications::dhcp::dhcp_parsing::DhcpMessage;
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        sockets::socket::{ProtocolFamily, SocketAddress, SocketType},
        user_process::{Application, ApplicationError, UserProcess},
        Sockets,
    },
    Control, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{Barrier, Notify};

// NOTE: THIS IS A TEMPORARY CLIENT
// TO BE DELETED ONCE DHCP HAS BEEN FULLY IMPLEMENTED ON THE CLIENT SIDE
#[derive(Default)]
pub struct DhcpClient {
    notify: Arc<Notify>,
    ip_address: Arc<RwLock<Option<Ipv4Address>>>,
}

impl DhcpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ip_address(&self) -> Ipv4Address {
        if let Some(ip_address) = *self.ip_address.read().unwrap() {
            ip_address
        } else {
            self.notify.notified().await;
            self.ip_address.read().unwrap().unwrap()
        }
    }

    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

impl Application for DhcpClient {
    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let sockets = protocols.protocol::<Sockets>().unwrap();
        let notify = self.notify.clone();
        let ip_address = self.ip_address.clone();
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
            let msg = socket.clone().recv_msg().await.unwrap();
            let parsed_msg = DhcpMessage::from_bytes(msg.iter()).unwrap();
            *ip_address.write().unwrap() = Some(parsed_msg.your_ip);
            notify.notify_waiters();
        });
        Ok(())
    }

    fn receive(
        &self,
        _message: Message,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
