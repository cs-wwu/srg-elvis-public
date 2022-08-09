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
use tokio::sync::mpsc::Sender;

/// An application that stores the first message it receives and then exits the
/// simulation.
#[derive(Clone)]
pub struct Forward {
    outgoing: Option<SharedSession>,
    local_ip: Ipv4Address,
    remote_ip: Ipv4Address,
    local_port: u16,
    remote_port: u16,
}

impl Forward {
    /// Creates a new capture.
    pub fn new(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            outgoing: Default::default(),
            local_ip,
            remote_ip,
            local_port,
            remote_port,
        }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(local_ip, remote_ip, local_port, remote_port))
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
        LocalAddress::set(&mut participants, self.local_ip);
        RemoteAddress::set(&mut participants, self.remote_ip);
        LocalPort::set(&mut participants, self.local_port);
        RemotePort::set(&mut participants, self.remote_port);
        let udp = context.protocol(Udp::ID).expect("No such protocol");
        let mut udp = udp.lock().unwrap();
        self.outgoing = Some(udp.open(Self::ID, participants.clone(), &mut context)?);
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
