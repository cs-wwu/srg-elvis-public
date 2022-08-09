use tokio::sync::mpsc::Sender;

use crate::{
    core::{message::Message, Control, ProtocolContext, ProtocolId, SharedSession},
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

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Default, Clone)]
pub struct Forward {
    outgoing: Option<SharedSession>,
}

impl Forward {
    /// Creates a new capture.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared() -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new())
    }
}

impl Application for Forward {
    const ID: ProtocolId = ProtocolId::from_string("Forward");

    fn start(
        &mut self,
        mut context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0xdeadu16);
        RemotePort::set(&mut participants, 0xbeefu16);
        let udp = context.protocol(Udp::ID).expect("No such protocol");
        let mut udp = udp.lock().unwrap();
        self.outgoing = Some(udp.open(Self::ID, participants.clone(), &mut context)?);
        LocalPort::set(&mut participants, 0xbeefu16);
        RemotePort::set(&mut participants, 0xdeadu16);
        udp.listen(Self::ID, participants, &mut context)?;
        Ok(())
    }

    fn recv(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.outgoing.as_mut().unwrap().send(message, context)?;
        Ok(())
    }
}
