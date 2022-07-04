use crate::core::{
    Control, ControlFlow, Message, Protocol, ProtocolContext, ProtocolId, RcSession, Session,
};
use std::error::Error;

pub struct Udp {}

impl Protocol for Udp {
    fn id(&self) -> ProtocolId {
        todo!()
    }

    fn open_active(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<RcSession, Box<dyn Error>> {
        todo!()
    }

    fn listen(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn demux(
        &mut self,
        message: Message,
        downstream: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        todo!()
    }
}

pub struct UdpSession {}

impl Session for UdpSession {
    fn protocol(&self) -> ProtocolId {
        todo!()
    }

    fn send(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn recv(
        &mut self,
        self_handle: RcSession,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn awake(
        &mut self,
        self_handle: RcSession,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
