use super::dhcp_client_listener::DhcpClientListener;
use super::dhcp_parsing::{DhcpMessage, MessageType};
use crate::protocols::{pci, Ipv4};
use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Endpoints, Udp},
    Control, Protocol, Session, Shutdown,
};
use pci::Pci;
use std::sync::{Arc, RwLock};
use tokio::sync::{Barrier, Notify};

/// An implementation of a DHCP client
#[derive(Default)]
pub struct DhcpClient {
    server_ip: Ipv4Address,
    notify: Arc<Notify>,
    listener: RwLock<Option<DhcpClientListener>>,
}

impl DhcpClient {
    pub fn new(server_ip: Ipv4Address, listen: Option<DhcpClientListener>) -> Self {
        Self {
            server_ip,
            notify: Default::default(),
            listener: RwLock::new(listen),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let server_ip = self.server_ip;
        // Wait on initialization before sending any message across the network

        initialized.wait().await;
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
            .open_and_listen(self.id(), sockets, protocols.clone())
            .await
            .unwrap();

        // Request an ip from the server
        // Todo(Justice): Have each slot request its own IP on their networks
        let response = DhcpMessage::default();
        let response_message = DhcpMessage::to_message(response).unwrap();
        udp.send(response_message, protocols).unwrap();

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let parsed_msg = DhcpMessage::from_bytes(message.iter()).unwrap();
        match parsed_msg.msg_type {
            MessageType::Offer => {
                let mut response = DhcpMessage::default();
                response.your_ip = parsed_msg.your_ip;
                response.msg_type = MessageType::Request;
                response.op = 2;
                caller
                    .send(DhcpMessage::to_message(response).unwrap(), protocols)
                    .unwrap();
                Ok(())
            }
            MessageType::Ack => {
                // Edit the receiving slot's ip_address
                let pci_demux_info = control
                    .get::<pci::DemuxInfo>()
                    .ok_or(DemuxError::MissingContext)?;

                let ipv4 = protocols.protocol::<Ipv4>().unwrap();
                let slot_index = ipv4
                    .contains(
                        protocols.protocol::<Pci>().unwrap().slot_count(),
                        pci_demux_info.slot,
                    )
                    .expect("No corresponding Ipv4Info struct found");
                ipv4.info.write().unwrap()[slot_index].ip_address = Some(parsed_msg.your_ip);
                self.notify.notify_waiters();
                
                // If listener application exists, call update and respond accordingly
                if self.listener.read().unwrap().is_some() {
                    if let Some(release) = self
                        .listener
                        .write()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .update(parsed_msg.your_ip)
                    {
                        caller
                            .send(DhcpMessage::to_message(release).unwrap(), protocols.clone())
                            .unwrap();
                        ipv4.info.write().unwrap()[slot_index].ip_address = None;
                        caller
                            .send(
                                DhcpMessage::to_message(DhcpMessage::default()).unwrap(),
                                protocols,
                            )
                            .unwrap();
                    }
                }
                Ok(())
            }
            _ => Err(DemuxError::Other),
        }
    }
}
