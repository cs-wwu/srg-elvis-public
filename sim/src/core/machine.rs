use crate::protocols::{Nic, NicError};

use super::{ArcProtocol, ControlFlow, MachineContext, ProtocolContext, ProtocolId};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

pub type ProtocolMap = Arc<HashMap<ProtocolId, ArcProtocol>>;

pub struct Machine {
    protocols: ProtocolMap,
    nic: Arc<RwLock<Nic>>,
}

impl Machine {
    pub fn new(nic: Arc<RwLock<Nic>>, protocols: impl Iterator<Item = ArcProtocol>) -> Self {
        // Todo: Guarantee that there are no duplicate protocols
        // Todo: Guarantee that the NIC is in there
        let nic_abstract: ArcProtocol = nic.clone();
        let protocols: HashMap<_, _> = protocols
            .chain(std::iter::once(nic_abstract))
            .map(|protocol| {
                let id = protocol.read().unwrap().id();
                (id, protocol)
            })
            .collect();
        Self {
            nic,
            protocols: Arc::new(protocols),
        }
    }

    pub fn awake(&mut self, context: &mut MachineContext) -> Result<ControlFlow, MachineError> {
        let protocol_context = ProtocolContext::new(self.protocols.clone());
        for message in context.pending() {
            self.nic
                .write()
                .unwrap()
                // Todo: We want to get the network number from pending()
                .accept_incoming(message, 0, protocol_context.clone())?;
        }

        let mut control_flow = ControlFlow::Continue;
        for protocol in self.protocols.values() {
            let flow = protocol.write().unwrap().awake(protocol_context.clone())?;
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
    #[error("The NIC protocol is missing")]
    MissingNic,
    #[error("{0}")]
    Nic(#[from] NicError),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
