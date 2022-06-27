use super::{AwakeContext, MachineContext, Protocol};
use std::{cell::RefCell, error::Error, rc::Rc};
use thiserror::Error as ThisError;

pub struct Machine {
    /// The first protocol should be the first to receive messages
    protocols: Vec<Rc<RefCell<dyn Protocol>>>,
}

// Todo: We need a way to make sure that the first protocol is a Nic. It would
// be ideal if the user just passed in the list of user programs they want to
// run and we handle the creation of the protocols they request at
// initialization time.

impl Machine {
    pub fn new(protocols: Vec<Rc<RefCell<dyn Protocol>>>) -> Self {
        Self { protocols }
    }

    pub fn awake(&mut self, context: &mut MachineContext) -> Result<(), MachineError> {
        let first = self.protocols.first().ok_or(MachineError::NoProtocols)?;
        for message in context.pending() {
            first.borrow_mut().demux(message)?;
        }

        let mut awake_context = AwakeContext::new(context);
        for protocol in self.protocols.iter() {
            protocol.borrow_mut().awake(&mut awake_context)?;
        }

        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum MachineError {
    #[error("Should have at least one protocol per machine")]
    NoProtocols,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
