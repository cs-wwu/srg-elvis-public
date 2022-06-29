use crate::core::{
    ArcProtocol, ArcSession, Control, ControlFlow, Message, NetworkLayer, Protocol,
    ProtocolContext, ProtocolId, Session,
};
use std::{
    error::Error,
    mem,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

#[derive(Default)]
pub struct Capture {
    sessions: Vec<Arc<RwLock<CaptureSession>>>,
}

impl Capture {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    pub fn new() -> Self {
        Default::default()
    }

    pub fn messages(&mut self) -> Vec<Message> {
        let mut messages = vec![];
        for session in self.sessions.iter() {
            messages.append(&mut session.write().unwrap().messages());
        }
        messages
    }
}

impl Protocol for Capture {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        _requester: ArcSession,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(Box::new(CaptureError::OpenActive))
    }

    fn open_passive(
        &mut self,
        requester: ArcSession,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let session = Arc::new(RwLock::new(CaptureSession::new(requester)));
        self.sessions.push(session.clone());
        Ok(session)
    }

    fn add_demux_binding(
        &mut self,
        _requester: ArcProtocol,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Err(Box::new(CaptureError::DemuxBinding))
    }

    fn demux(&self, _message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Err(Box::new(CaptureError::Demux))
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        for session in self.sessions.iter_mut() {
            session.write().unwrap().awake(context.clone())?;
        }
        Ok(ControlFlow::Continue)
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
            received: vec![],
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

    fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        self.downstream.write().unwrap().send(message)
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
    #[error("Attempted an active open on a capture protocol")]
    OpenActive,
    #[error("Attempted a demux binding on a capture protocol")]
    DemuxBinding,
    #[error("Attempted demuxing with a capture protocol")]
    Demux,
}
