use elvis_core::{
    gcd::GcdHandle,
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Udp,
    },
    Control, Id, ProtocolMap,
};
use std::sync::{Arc, RwLock};

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Debug)]
pub struct Capture {
    /// The message that was received, if any
    message: RwLock<Option<Message>>,
    gcd: RwLock<Option<GcdHandle>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    #[allow(unused)]
    port: u16,
    /// The number of messages it will receive before stopping
    message_count: u32,
    /// The number of messages currently recieved
    cur_count: RwLock<u32>,
    // / The transport protocol to use
    // transport: Transport,
}

impl Capture {
    /// Creates a new capture.
    pub fn new(ip_address: Ipv4Address, port: u16, message_count: u32) -> Self {
        Self {
            message: Default::default(),
            gcd: Default::default(),
            ip_address,
            port,
            message_count,
            cur_count: RwLock::new(0),
            // transport: Transport::Udp,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    /// Gets the message that was received.
    pub fn message(&self) -> Option<Message> {
        self.message.read().unwrap().clone()
    }

    // / Set the transport protocol to use
    // pub fn transport(mut self, transport: Transport) -> Self {
    //     self.transport = transport;
    //     self
    // }
}

impl Application for Capture {
    const ID: Id = Id::from_string("Capture");

    fn start(&self, gcd: GcdHandle, protocols: ProtocolMap) -> Result<(), ApplicationError> {
        *self.gcd.write().unwrap() = Some(gcd);
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);

        Udp::set_local_port(self.port, &mut participants);
        // match self.transport {
        //     Transport::Udp => Udp::set_local_port(self.port, &mut participants),
        //     Transport::Tcp => Tcp::set_local_port(self.port, &mut participants),
        // }

        protocols
            // .protocol(self.transport.id())
            .protocol(Udp::ID)
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)?;
        Ok(())
    }

    fn receive(&self, message: Message, _context: Context) -> Result<(), ApplicationError> {
        *self.message.write().unwrap() = Some(message);
        *self.cur_count.write().unwrap() += 1;
        if *self.cur_count.read().unwrap() >= self.message_count {
            if let Some(gcd) = self.gcd.write().unwrap().take() {
                gcd.shut_down();
            }
        }
        Ok(())
    }
}
