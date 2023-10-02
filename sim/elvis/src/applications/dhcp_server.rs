use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        dhcp::dhcp_parsing::{DhcpMessage, MessageType},
        ipv4::Ipv4Address,
        Endpoint, Udp,
    },
    Control, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

use crate::ip_generator::*;

/// A struct describing an implementation of a DHCP server
pub struct DhcpServer {
    server_address: Ipv4Address,
    pub ip_generator: RwLock<IpGenerator>,
}

impl DhcpServer {
    pub fn new(server_address: Ipv4Address, ip_range: IpRange) -> Self {
        Self {
            server_address,
            ip_generator: RwLock::new(IpGenerator::new(ip_range)),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpServer {
    /// Initialize the server and listen/respond to client requests
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().unwrap();
        udp.listen(self.id(), Endpoint::new(self.server_address, 67), protocols)
            .unwrap();
        initialized.wait().await;
        Ok(())
    }

    /// Respond to DHCP messages in correspondence with RFC 2131
    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let message = DhcpMessage::from_bytes(message.iter()).unwrap();
        match message.msg_type {
            MessageType::Discover => {
                let mut response = DhcpMessage::default();
                // Todo: Gracefully handle the case of no addresses available
                response.your_ip = self.ip_generator.write().unwrap().fetch_ip().unwrap();
                response.op = 2;
                response.msg_type = MessageType::Offer;
                let response = DhcpMessage::to_message(response).unwrap();
                caller.send(response, protocols).unwrap();
                Ok(())
            }
            MessageType::Request => {
                let mut response = DhcpMessage::default();
                response.op = 2;
                response.your_ip = message.your_ip;
                response.msg_type = MessageType::Ack;
                let response = DhcpMessage::to_message(response).unwrap();
                caller.send(response, protocols).unwrap();
                Ok(())
            }
            MessageType::Release => {
                self.ip_generator
                    .write()
                    .unwrap()
                    .return_ip(message.your_ip);
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
