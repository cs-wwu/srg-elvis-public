use crate::protocols::{Tap, TapError};

use super::{ControlFlow, MachineContext, ProtocolContext, ProtocolId, RcProtocol};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    error::Error,
    rc::Rc,
};
use thiserror::Error as ThisError;

pub type ProtocolMap = Rc<HashMap<ProtocolId, RcProtocol>>;

pub struct Machine {
    protocols: ProtocolMap,
    tap: Rc<RefCell<Tap>>,
}

impl Machine {
    pub fn new(
        tap: Rc<RefCell<Tap>>,
        protocols: impl Iterator<Item = RcProtocol>,
    ) -> Result<Self, MachineError> {
        let tap_abstract: RcProtocol = tap.clone();
        let mut map = HashMap::new();
        for protocol in protocols.chain(std::iter::once(tap_abstract)) {
            let id = protocol.borrow().id();
            match map.entry(id) {
                Entry::Occupied(_) => Err(MachineError::DuplicateProtocol)?,
                Entry::Vacant(entry) => {
                    entry.insert(protocol);
                }
            }
        }
        Ok(Self {
            tap,
            protocols: Rc::new(map),
        })
    }

    pub fn awake(&mut self, context: &mut MachineContext) -> Result<ControlFlow, MachineError> {
        let protocol_context = ProtocolContext::new(self.protocols.clone());
        for message in context.pending() {
            self.tap
                .borrow_mut()
                // Todo: We want to get the network number from pending()
                .accept_incoming(message, 0, protocol_context.clone())?;
        }

        let mut control_flow = ControlFlow::Continue;
        for protocol in self.protocols.values() {
            let flow = protocol.borrow_mut().awake(protocol_context.clone())?;
            match flow {
                ControlFlow::Continue => {}
                ControlFlow::EndSimulation => control_flow = ControlFlow::EndSimulation,
            }
        }

        Ok(control_flow)
    }
}

#[derive(Debug, ThisError)]
pub enum MachineError {
    #[error("Only one of each protocol should be provided")]
    DuplicateProtocol,
    #[error("The Tap protocol is missing")]
    MissingTap,
    #[error("{0}")]
    Tap(#[from] TapError),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
