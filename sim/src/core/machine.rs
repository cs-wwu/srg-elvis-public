use crate::protocols::Nic;

use super::{ArcProtocol, MachineContext, ProtocolContext, ProtocolId};
use std::{collections::HashMap, error::Error, sync::Arc};
use thiserror::Error as ThisError;

// Todo: Take network as a protocol that lives at the bottom of the protocol
// stack

pub type ProtocolMap = Arc<HashMap<ProtocolId, ArcProtocol>>;

pub struct Machine {
    /// The first protocol should be the first to receive messages
    protocols: ProtocolMap,
}

// Todo: We need a way to make sure that the first protocol is a Nic. It would
// be ideal if the user just passed in the list of user programs they want to
// run and we handle the creation of the protocols they request at
// initialization time.

impl Machine {
    pub fn new(protocols: impl Iterator<Item = ArcProtocol>) -> Self {
        // Todo: Guarantee that there are no duplicate protocols
        // Todo: Guarantee that the NIC is in there
        let protocols: HashMap<_, _> = protocols
            .map(|protocol| {
                let id = protocol.read().unwrap().id();
                (id, protocol)
            })
            .collect();
        Self {
            protocols: Arc::new(protocols),
        }
    }

    pub fn awake(&mut self, context: &mut MachineContext) -> Result<(), MachineError> {
        let nic = self
            .protocols
            .get(&Nic::ID)
            .ok_or(MachineError::MissingNic)?;
        let protocol_context = ProtocolContext::new(self.protocols.clone());
        for message in context.pending() {
            nic.read()
                .unwrap()
                .demux(message, protocol_context.clone())?;
        }

        for protocol in self.protocols.values() {
            protocol.write().unwrap().awake(protocol_context.clone())?;
        }

        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum MachineError {
    #[error("The NIC protocol is missing")]
    MissingNic,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
