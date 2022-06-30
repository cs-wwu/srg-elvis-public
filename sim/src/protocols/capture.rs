use crate::core::{
    ArcSession, Control, ControlFlow, Message, NetworkLayer, Protocol, ProtocolContext, ProtocolId,
    Session,
};
use std::{
    error::Error,
    mem,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

pub struct Capture {
    session: Arc<RwLock<CaptureSession>>,
}

impl Capture {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    pub fn new(downstream: ArcSession) -> Self {
        Self {
            session: Arc::new(RwLock::new(CaptureSession::new(downstream))),
        }
    }

    pub fn messages(&mut self) -> Vec<Message> {
        self.session.write().unwrap().messages()
    }
}

impl Protocol for Capture {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(Box::new(CaptureError::OpenActive))
    }

    fn open_passive(
        &mut self,
        _downstream: ArcSession,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Ok(self.session.clone())
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(Box::new(CaptureError::DemuxBinding))
    }

    fn demux(&self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        self.session.write().unwrap().recv(message, context)
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        self.session.write().unwrap().awake(context)?;
        // Todo: If Control is going to a useful debugging tool, we probably want a more
        // robust choice of whether to end the simulation than just asking whether we
        // have received any messages.
        if self.session.read().unwrap().received.is_empty() {
            Ok(ControlFlow::Continue)
        } else {
            Ok(ControlFlow::EndSimulation)
        }
    }
}

pub struct CaptureSession {
    downstream: ArcSession,
    received: Vec<Message>,
}

impl CaptureSession {
    fn new(downstream: ArcSession) -> Self {
        Self {
            downstream,
            received: Default::default(),
        }
    }

    pub fn messages(&mut self) -> Vec<Message> {
        mem::take(&mut self.received)
    }
}

impl Session for CaptureSession {
    fn protocol(&self) -> ProtocolId {
        Capture::ID
    }

    fn send(&mut self, message: Message, context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        self.downstream.write().unwrap().send(message, context)
    }

    fn recv(&mut self, message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        self.received.push(message);
        Ok(())
    }

    fn awake(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum CaptureError {
    #[error("There is not an active capture session")]
    NoSession,
    #[error("Attempted an active open on a capture protocol")]
    OpenActive,
    #[error("Attempted a demux binding on a capture protocol")]
    DemuxBinding,
    #[error("Attempted demuxing with a capture protocol")]
    Demux,
}
