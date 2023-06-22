use super::udp_parsing::build_udp_header;
use crate::{
    logging::{receive_message_event, send_message_event},
    machine::ProtocolMap,
    message::Message,
    protocol::DemuxError,
    protocols::utility::Endpoints,
    session::SendError,
    Control, Session,
};
use std::{any::TypeId, fmt::Debug, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: TypeId,
    pub downstream: Arc<dyn Session>,
    pub sockets: Endpoints,
}

impl UdpSession {
    pub fn receive(
        self: Arc<Self>,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        receive_message_event(
            self.sockets.local.address,
            self.sockets.remote.address,
            self.sockets.local.port,
            self.sockets.remote.port,
            message.clone(),
        );
        protocols
            .get(self.upstream)
            .expect("No such protocol")
            .demux(message, self, control, protocols)?;
        Ok(())
    }
}

impl Session for UdpSession {
    fn send(&self, mut message: Message, protocols: ProtocolMap) -> Result<(), SendError> {
        let id = self.sockets;
        // TODO(hardint): Should this fail or just segment the message into
        // multiple IP packets?
        let header = match build_udp_header(
            self.sockets.local.address,
            id.local.port,
            self.sockets.remote.address,
            id.remote.port,
            message.iter(),
            message.len(),
        ) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(SendError::Header)?
            }
        };
        send_message_event(
            self.sockets.local.address,
            self.sockets.remote.address,
            id.local.port,
            id.remote.port,
            message.clone(),
        );
        message.header(header);
        self.downstream.send(message, protocols)?;
        Ok(())
    }
}

impl Debug for UdpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpSession")
            .field("id", &self.sockets)
            .finish()
    }
}
