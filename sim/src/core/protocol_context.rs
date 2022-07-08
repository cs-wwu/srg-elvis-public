use super::{protocol::RcProtocol, Control, ProtocolId, ProtocolMap};

#[derive(Clone, Default)]
pub struct ProtocolContext {
    protocols: ProtocolMap,
    info: Control,
}

impl ProtocolContext {
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::default(),
        }
    }

    pub fn protocol(&self, id: ProtocolId) -> Option<RcProtocol> {
        self.protocols.get(&id).cloned()
    }

    pub fn info(&mut self) -> &mut Control {
        &mut self.info
    }
}
