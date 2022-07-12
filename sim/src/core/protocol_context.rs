use super::{protocol::RcProtocol, Control, ProtocolId, ProtocolMap, SharedSession};

/// Provides a [`Protocol`](super::Protocol) with information about its
/// execution environment.
#[derive(Clone)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
    session_stack: Vec<SharedSession>,
    /// A key-value store for exchanging unstructured information between
    /// [`Protocol`](super::Protocol)s.
    pub info: Control,
}

impl ProtocolContext {
    /// Create a new protocol context.
    pub(super) fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::new(),
            session_stack: vec![],
        }
    }

    /// Get a handle to the protocol identified by `id`.
    pub fn protocol(&self, id: ProtocolId) -> Option<RcProtocol> {
        self.protocols.get(&id).cloned()
    }

    /// Get a handle to the currently executing [`Session`](super::Session).
    pub fn current_session(&mut self) -> Option<SharedSession> {
        self.session_stack.last().cloned()
    }

    /// Add a new session to the top of the list of currently executing
    /// [`Session`](super::Session)s.
    pub(super) fn push_session(&mut self, session: SharedSession) {
        self.session_stack.push(session)
    }

    /// Remove the topmost currently executing [`Session`](super::Session)s.
    pub(super) fn pop_session(&mut self) {
        self.session_stack.pop();
    }
}
