use super::{protocol::RcProtocol, Control, ProtocolId, ProtocolMap, SharedSession};

#[derive(Clone)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
    session_stack: Vec<SharedSession>,
    pub info: Control,
}

impl ProtocolContext {
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::new(),
            session_stack: vec![],
        }
    }

    pub fn protocol(&self, id: ProtocolId) -> Option<RcProtocol> {
        self.protocols.get(&id).cloned()
    }

    pub fn current_session(&mut self) -> Option<SharedSession> {
        self.session_stack.last().cloned()
    }

    pub fn push_session(&mut self, session: SharedSession) {
        self.session_stack.push(session)
    }

    pub fn pop_session(&mut self) {
        self.session_stack.pop();
    }
}
