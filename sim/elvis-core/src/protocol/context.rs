use super::SharedProtocol;
use crate::{machine::ProtocolMap, Control, Id};

/// Provides a [`Protocol`](super::Protocol) with information about its
/// execution environment.
#[derive(Clone)]
pub struct Context {
    pub protocols: ProtocolMap,
    /// A key-value store for exchanging unstructured information between
    /// [`Protocol`](super::Protocol)s.
    pub control: Control,
}

impl Context {
    /// Create a new protocol context.
    pub fn new(protocols: ProtocolMap) -> Self {
        Self {
            protocols,
            control: Control::new(),
        }
    }

    /// Get a handle to the protocol identified by `id`.
    pub fn protocol(&self, id: Id) -> Option<SharedProtocol> {
        self.protocols.protocol(id)
    }
}
