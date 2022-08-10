use tokio::sync::mpsc::Sender;

use super::{tap_misc::TapError, NetworkId};
use crate::core::{
    message::Message, MachineId, PhysicalAddress, Postmarked, ProtocolContext, ProtocolId, Session,
};
use std::{error::Error, sync::Arc};

#[derive(Clone)]
pub struct TapSession {
    upstream: ProtocolId,
    machine_id: MachineId,
    sender: Sender<Postmarked>,
}

impl TapSession {
    pub(super) fn new(
        upstream: ProtocolId,
        machine_id: MachineId,
        sender: Sender<Postmarked>,
    ) -> Self {
        Self {
            upstream,
            machine_id,
            sender,
        }
    }
}

impl Session for TapSession {
    fn send(
        self: Arc<Self>,
        message: Message,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let message = message.with_header(&self.upstream.into_inner().to_be_bytes());
        let postmarked = Postmarked {
            message,
            // TODO(hardint): Replace with correct destination
            address: PhysicalAddress::Broadcast,
            sender: self.machine_id,
        };
        let sender = self.sender.clone();
        tokio::spawn(async move {
            sender.send(postmarked).await.unwrap();
            println!("Sending");
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        println!("Receiving");
        let protocol = context
            .protocol(self.upstream)
            .ok_or(TapError::NoSuchProtocol(self.upstream))?;
        protocol.demux(message, self, context)
    }

    fn start(self: Arc<Self>, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    upstream: ProtocolId,
    network: NetworkId,
}

impl SessionId {
    pub fn new(upstream: ProtocolId, network: NetworkId) -> Self {
        Self { upstream, network }
    }
}
