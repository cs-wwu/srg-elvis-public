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
    session: Option<Arc<RwLock<CaptureSession>>>,
}

impl Capture {
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    pub fn new() -> Self {
        Default::default()
    }

    pub fn messages(&mut self) -> Vec<Message> {
        match &self.session {
            Some(session) => session.write().unwrap().messages(),
            None => vec![],
        }
    }
}

impl Protocol for Capture {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        _requester: ArcProtocol,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(Box::new(CaptureError::OpenActive))
    }

    fn open_passive(
        &mut self,
        requester: ArcProtocol,
        identifier: Control,
        context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let requester = context.session(requester.read().unwrap().id(), identifier)?;
        Ok(match &self.session {
            Some(session) => session.clone(),
            None => {
                let session = Arc::new(RwLock::new(CaptureSession::new(requester)));
                self.session = Some(session.clone());
                session
            }
        })
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
        if let Some(session) = &self.session {
            session.write().unwrap().awake(context.clone())?;
        }
        Ok(ControlFlow::Continue)
    }

    fn get_session(&self, _identifier: &Control) -> Result<ArcSession, Box<dyn Error>> {
        Ok(self
            .session
            .as_ref()
            .ok_or(CaptureError::NoSession)?
            .clone())
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
    #[error("There is not an active capture session")]
    NoSession,
    #[error("Attempted an active open on a capture protocol")]
    OpenActive,
    #[error("Attempted a demux binding on a capture protocol")]
    DemuxBinding,
    #[error("Attempted demuxing with a capture protocol")]
    Demux,
}
