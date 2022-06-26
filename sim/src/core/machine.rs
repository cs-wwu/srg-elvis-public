use std::rc::Rc;

use super::{MachineContext, Protocol};

pub struct Machine {
    /// The first protocol should be the first to receive messages
    protocols: Vec<Rc<dyn Protocol>>,
}

impl Machine {
    pub fn new(protocols: Vec<Rc<dyn Protocol>>) -> Self {
        Self { protocols }
    }

    pub fn awake(&mut self, context: &mut MachineContext) {
        todo!()
    }
}
