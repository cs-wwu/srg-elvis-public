use crate::{machine::ProtocolMap, Control};

use super::{ProtocolId, SharedProtocol};

/// Provides a [`Protocol`](super::Protocol) with information about its
/// execution environment.
#[derive(Clone)]
pub struct Context {
    pub protocols: ProtocolMap,
    /// A key-value store for exchanging unstructured information between
    /// [`Protocol`](super::Protocol)s.
    pub info: Control,
}

impl Context {
    /// Create a new protocol context.
    pub(crate) fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            info: Control::new(),
        }
    }

    /// Get a handle to the protocol identified by `id`.
    pub fn protocol(&self, id: ProtocolId) -> Option<SharedProtocol> {
        self.protocols.get(&id).cloned()
    }
}
