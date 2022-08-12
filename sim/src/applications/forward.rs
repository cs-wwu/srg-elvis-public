use crate::{
    core::{
        message::Message, protocol::ProtocolId, session::SharedSession, Control, ProtocolContext,
    },
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

/// An application that forwards messages to `local_ip` to `remote_ip`.
#[derive(Clone)]
pub struct Forward {
    /// The session on which we send any messages we receive
    outgoing: Arc<Mutex<Option<SharedSession>>>,
    /// The IP address for incoming messages
    local_ip: Ipv4Address,
    /// The IP address for outgoing messages
    remote_ip: Ipv4Address,
    /// The port number for incoming messages
    local_port: u16,
    /// The port number for outgoing messages
    remote_port: u16,
}

impl Forward {
    /// Creates a new forwarding application.
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

    /// Creates a new forwarding application behind a shared handle.
    pub fn new_shared(
        local_ip: Ipv4Address,
        remote_ip: Ipv4Address,
        local_port: u16,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(local_ip, remote_ip, local_port, remote_port))
    }
}

impl Application for Forward {
    const ID: ProtocolId = ProtocolId::from_string("Forward");

    fn start(
        self: Arc<Self>,
        context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, self.local_ip);
        RemoteAddress::set(&mut participants, self.remote_ip);
        LocalPort::set(&mut participants, self.local_port);
        RemotePort::set(&mut participants, self.remote_port);
        let udp = context.protocol(Udp::ID).expect("No such protocol");
        {
            *self.outgoing.lock().unwrap() = Some(udp.clone().open(
                Self::ID,
                // TODO(hardint): Can these clones be cheaper?
                participants.clone(),
                context.clone(),
            )?);
        }
        udp.listen(Self::ID, participants, context)?;
        Ok(())
    }

    fn recv(
        self: Arc<Self>,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.outgoing
            .clone()
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
            .send(message, context)?;
        Ok(())
    }
}
