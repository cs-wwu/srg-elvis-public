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
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

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
                response.your_ip = self.ip_generator.write().unwrap().fetch_ip();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpRange {
    pub start: Ipv4Address,
    pub end: Ipv4Address,
}

impl IpRange {
    pub fn new(start: Ipv4Address, end: Ipv4Address) -> Self {
        Self { start, end }
    }
}

#[derive(Debug)]
pub struct IpGenerator {
    pub current: u32,
    pub end: u32,
    returned_ips: VecDeque<Ipv4Address>,
}

impl IpGenerator {
    pub fn new(range: IpRange) -> Self {
        Self {
            current: range.start.into(),
            end: range.end.into(),
            returned_ips: VecDeque::<Ipv4Address>::new(),
        }
    }

    //TODO: Handle no available addresses in returned & ipGen
    fn fetch_ip(&mut self) -> Ipv4Address {
        if self.returned_ips.is_empty() {
            self.next().unwrap()
        } else {
            self.returned_ips.pop_front().unwrap()
        }
    }

    fn return_ip(&mut self, returned: Ipv4Address) {
        self.returned_ips.push_back(returned)
    }
}

impl Iterator for IpGenerator {
    type Item = Ipv4Address;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let out = self.current.into();
            self.current += 1;
            Some(out)
        }
    }
}
