use super::{ProtocolId, SharedProtocol};
use crate::{machine::ProtocolMap, Control};

/// Provides a [`Protocol`](super::Protocol) with information about its
/// execution environment.
#[derive(Clone)]
pub struct Context {
    protocols: ProtocolMap,
    /// A key-value store for exchanging unstructured information between
    /// [`Protocol`](super::Protocol)s.
    pub info: Control,
}

impl Context {
    /// Create a new protocol context.
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::new(),
        }
    }

    /// Get a handle to the protocol identified by `id`.
    pub fn protocol(&self, id: ProtocolId) -> Option<SharedProtocol> {
        self.protocols.protocol(id)
    }
}
