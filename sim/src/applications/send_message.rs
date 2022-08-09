use tokio::sync::mpsc::Sender;

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
    local_ip: Ipv4Address,
    remote_ip: Ipv4Address,
    local_port: u16,
    remote_port: u16,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(
        text: &'static str,
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Self {
        Self {
            text,
            local_ip,
            remote_ip,
            local_port,
            remote_port,
        }
    }

    /// Creates a new send message application behind a shared handle.
    pub fn new_shared(
        text: &'static str,
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Arc<Mutex<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(
            text,
            local_ip,
            remote_ip,
            local_port,
            remote_port,
        ))
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
        LocalAddress::set(&mut participants, self.local_ip);
        RemoteAddress::set(&mut participants, self.remote_ip);
        LocalPort::set(&mut participants, self.local_port);
        RemotePort::set(&mut participants, self.remote_port);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let mut session = protocol
            .lock()
            .unwrap()
            .open(Self::ID, participants, &mut context)?;
        session.send(Message::new(self.text), &mut context)?;
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
