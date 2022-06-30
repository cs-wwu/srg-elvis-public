use crate::core::{
    ArcSession, Control, ControlFlow, Message, Protocol, ProtocolContext, ProtocolId, Session,
};
use std::error::Error;

pub struct Ip {}

impl Protocol for Ip {
    fn id(&self) -> ProtocolId {
        todo!()
    }

    fn open_active(
        &mut self,
        requester: ProtocolId,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        todo!()
    }

    fn open_passive(
        &mut self,
        requester: ProtocolId,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        todo!()
    }

    fn add_demux_binding(
        &mut self,
        requester: ProtocolId,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        todo!()
    }

    fn get_session(&self, identifier: &Control) -> Result<ArcSession, Box<dyn Error>> {
        todo!()
    }
}

pub struct IpSession {}

impl Session for IpSession {
    fn protocol(&self) -> ProtocolId {
        todo!()
    }

    fn send(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn recv(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
