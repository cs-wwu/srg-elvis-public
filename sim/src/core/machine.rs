use std::collections::VecDeque;

use super::{Protocol, Message, MachineContext};

pub struct Machine {
    protocols: Vec<Box<dyn Protocol>>,
}

impl Machine {
    pub fn awake(&mut self, context: &mut MachineContext) {
        todo!()
    }
}