use super::udp_parsing::build_udp_header;
use crate::{
    logging::{receive_message_event, send_message_event},
    message::Message,
    protocol::DemuxError,
    protocols::utility::Endpoints,
    session::SendError,
    Control, Machine, Session,
};
use std::{any::TypeId, fmt::Debug, sync::Arc};

pub(super) struct UdpSession {
    pub upstream: TypeId,
    pub downstream: Arc<dyn Session>,
    pub endpoints: Endpoints,
}

impl UdpSession {
    pub fn receive(
        self: Arc<Self>,
        message: Message,
        control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        receive_message_event(
            self.endpoints.local.address,
            self.endpoints.remote.address,
            self.endpoints.local.port,
            self.endpoints.remote.port,
            message.clone(),
        );
        machine
            .get(self.upstream)
            .expect("No such protocol")
            .demux(message, self, control, machine)?;
        Ok(())
    }
}

impl Session for UdpSession {
    fn send(&self, mut message: Message, machine: Arc<Machine>) -> Result<(), SendError> {
        let id = self.endpoints;
        // TODO(hardint): Should this fail or just segment the message into
        // multiple IP packets?
        let header = match build_udp_header(
            self.endpoints.local.address,
            id.local.port,
            self.endpoints.remote.address,
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
            self.endpoints.local.address,
            self.endpoints.remote.address,
            id.local.port,
            id.remote.port,
            message.clone(),
        );
        message.header(header);
        self.downstream.send(message, machine)?;
        Ok(())
    }
}

impl Debug for UdpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpSession")
            .field("id", &self.endpoints)
            .finish()
    }
}
