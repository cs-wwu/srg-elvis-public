use super::{protocol::RcProtocol, Control, ProtocolId, ProtocolMap, SharedSession};

#[derive(Clone)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
    pub info: Control,
    pub current_session: Option<SharedSession>,
}

impl ProtocolContext {
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::new(),
            current_session: None,
        }
    }

    pub fn protocol(&self, id: ProtocolId) -> Option<RcProtocol> {
        self.protocols.get(&id).cloned()
    }
}
