use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};

use crate::{
    core::{message::Message, Control, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

/// An application that sends a single message over the network.
pub struct SendMessage {
    text: &'static str,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(text: &'static str) -> Self {
        Self { text }
    }

    /// Creates a new send message application behind a shared handle.
    pub fn new_shared(text: &'static str) -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(text))
    }
}

impl Application for SendMessage {
    const ID: ProtocolId = ProtocolId::from_string("Send Message");

    fn start(
        &mut self,
        mut context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        let mut participants = Control::new();
        // TODO(hardint): This should be some other IP address
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0xdeadu16);
        RemotePort::set(&mut participants, 0xbeefu16);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let mut session = protocol
            .lock()
            .unwrap()
            .open(Self::ID, participants, &mut context)?;
        // Wait for one second to make sure that the server has started up
        let text = <&str>::clone(&self.text);
        tokio::spawn(async move {
            sleep(Duration::from_millis(1000)).await;
            session.send(Message::new(text), &mut context).unwrap();
        });
        Ok(())
    }

    fn recv(
        &mut self,
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
