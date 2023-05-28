use crate::applications::dhcp::dhcp_parsing::DhcpMessage;
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Endpoint, Endpoints, Pci, Udp,
    },
    Control, Session, Shutdown,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
use tokio::sync::{Barrier, Notify};

// NOTE: THIS IS A TEMPORARY CLIENT
// TO BE DELETED ONCE DHCP HAS BEEN FULLY IMPLEMENTED ON THE CLIENT SIDE
#[derive(Default)]
pub struct DhcpClient {
    server_ip: Ipv4Address,
    notify: Arc<Notify>,
    ip_address: Arc<RwLock<Option<Ipv4Address>>>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            ip_address: Default::default(),
        }
    }

    pub async fn ip_address(&self) -> Ipv4Address {
        if let Some(ip_address) = *self.ip_address.read().unwrap() {
            return ip_address;
        }
        self.notify.notified().await;
        self.ip_address.read().unwrap().unwrap()
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
        let server_ip = self.server_ip;
        tokio::spawn(async move {
            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            // NOTE(hardint):
            // The problem I'm having using sockets here at the moment is that the connect method
            // expects that we already have a local address. The tests are specifying a local
            // address for sockets in the constructor, but that doesn't really make sense for the
            // test since that's what DHCP is supposed to be doing. I think that sockets will have
            // to support opening a connection that doesn't have a local address before it is a
            // viable interface for this protocol. According to ChatGPT, we should use UDP with a
            // local address of 0.0.0.0, port 68 for the client, and port 67 for the server. Some
            // unwraps in here can probably be handled better.

            let sockets = Endpoints {
                local: Endpoint {
                    address: Ipv4Address::new([0, 0, 0, 0]),
                    port: 68,
                },
                remote: Endpoint {
                    address: server_ip,
                    port: 67,
                },
            };
            let udp = protocols
                .protocol::<Udp>()
                .unwrap()
                .open(
                    TypeId::of::<UserProcess<Self>>(),
                    sockets,
                    protocols.clone(),
                )
                .unwrap();
            let response = DhcpMessage::default();
            let response_message = DhcpMessage::to_message(response).unwrap();
            udp.send(response_message, protocols).unwrap();
        });
        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let parsed_msg = DhcpMessage::from_bytes(message.iter()).unwrap();
        let macs: Vec<_> = protocols
            .protocol::<Pci>()
            .unwrap()
            .mac_addresses()
            .collect();
        println!("DHCP Client got {} on MAC {}", parsed_msg.your_ip, macs[0]);
        *self.ip_address.write().unwrap() = Some(parsed_msg.your_ip);
        self.notify.notify_waiters();
        Ok(())
    }
}
