use super::dhcp_parsing::{DhcpMessage, MessageType};
use crate::{
    machine::ProtocolMap,
    protocol::{DemuxError, StartError},
    protocols::ipv4::Ipv4Address,
    Control, Message, Protocol, Session, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

/// A struct designed to observe a DhcpClient and respond to address acquisition
/// Specifically for testing Release functionality as it stands
#[derive(Debug, Default)]
pub struct DhcpClientListener {
    pub first_ip: Option<Ipv4Address>,
    pub second_ip: Option<Ipv4Address>,
}

impl DhcpClientListener {
    pub fn new() -> Self {
        Self {
            first_ip: None,
            second_ip: None,
        }
    }

    /// Respond to a client acquiring a new IP address, telling them whether to release
    pub fn update(&mut self, addr: Ipv4Address) -> Option<DhcpMessage> {
        match self.first_ip {
            None => {
                self.first_ip = Some(addr);
                let mut response = DhcpMessage::default();
                response.your_ip = addr;
                response.msg_type = MessageType::Release;
                Some(response)
            }
            Some(_addr) => {
                self.second_ip = Some(addr);
                assert_eq!(self.first_ip.unwrap(), self.second_ip.unwrap());
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl Protocol for DhcpClientListener {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}
