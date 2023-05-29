use super::dhcp_parsing::{DhcpMessage, MessageType};
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError},
        Endpoint, Udp, UserProcess,
    },
    Control, Session, Shutdown,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

// Port number & broadcast frequency used by DHCP servers
pub const PORT_NUM: u16 = 67;
pub const BROADCAST: Ipv4Address = Ipv4Address::new([255, 255, 255, 255]);

/// A struct describing an implementation of a DHCP server
pub struct DhcpServer {
    server_address: Ipv4Address,
    ip_generator: RwLock<IpGenerator>,
}

impl DhcpServer {
    pub fn new(server_address: Ipv4Address, ip_range: IpRange) -> Self {
        Self {
            server_address,
            ip_generator: RwLock::new(IpGenerator::new(ip_range)),
        }
    }

    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

// We should move this into application in the 'elvis' branch eventually
impl Application for DhcpServer {
    /// Initialize the server and listen/respond to client requests
    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // NOTE(hardint):
        // Just the same as with the client side, I don't think that sockets works here. The
        // problem is that every client requesting an IP address has the same set of endpoints so
        // sockets interprets them as all belonging to the same connection and won't hand out
        // addresses after the first. I'm modifying this to use UDP instead.

        let udp = protocols.protocol::<Udp>().unwrap();
        udp.listen(
            TypeId::of::<UserProcess<Self>>(),
            Endpoint::new(self.server_address, 67),
            protocols,
        )
        .unwrap();
        tokio::spawn(async move {
            initialized.wait().await;
        });
        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // NOTE(hardint):
        //
        // The way this is implemented does not seem to be conformant. The code generates a new IP
        // address for the client when it sends a discover message, but that should be what happens
        // in response to a request message. ChatGPT summary:
        //
        // DHCP Discover:
        //     The DHCP Discover message is broadcasted by a client when it first joins
        //     a network or when it needs to renew its lease.
        //
        //     The client sends a DHCP Discover message to discover available DHCP servers in the
        //     network.
        //
        //     The source IP address in the DHCP Discover message is typically set to 0.0.0.0, and
        //     the destination IP address is set to the limited broadcast address
        //     (255.255.255.255).
        //
        //     The client includes a list of requested parameters (such as IP address, subnet mask,
        //     gateway, DNS servers, etc.) that it would like to receive from the DHCP server.
        //
        // DHCP Request:
        //
        //     After receiving DHCP Offer messages from one or more DHCP servers, the
        //     client chooses a DHCP server and sends a DHCP Request message.
        //
        //     The DHCP Request message is used by the client to formally request the offered IP
        //     address and other network configuration parameters from the selected DHCP server.
        //
        //     The source IP address in the DHCP Request message is typically set to 0.0.0.0, and
        //     the destination IP address is set to the IP address of the selected DHCP server.
        //
        //     The client includes the specific IP address offered by the DHCP server within the
        //     DHCP Request message. If multiple DHCP servers responded with offers, the client
        //     includes the DHCP server's IP address in the DHCP Request message to identify the
        //     server from which it is accepting the offer.

        let message = DhcpMessage::from_bytes(message.iter()).unwrap();
        match message.msg_type {
            MessageType::Discover => {
                let mut response = DhcpMessage::default();
                // Todo: Gracefully handle the case of no addresses available
                response.your_ip = self.ip_generator.write().unwrap().next().unwrap();
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
                response.msg_type = MessageType::Offer;
                let response = DhcpMessage::to_message(response).unwrap();
                caller.send(response, protocols).unwrap();
                Ok(())
            }
            // TODO: Invalid message type, send error
            _ => Err(ApplicationError::Other),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpGenerator {
    pub current: u32,
    pub end: u32,
}

impl IpGenerator {
    pub fn new(range: IpRange) -> Self {
        Self {
            current: range.start.into(),
            end: range.end.into(),
        }
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
