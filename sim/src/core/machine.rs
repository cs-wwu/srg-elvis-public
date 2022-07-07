use crate::protocols::tap::Tap;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use super::{
    internet::MachineContext,
    network::PhysicalAddress,
    protocol::{ControlFlow, ProtocolContext, RcProtocol},
    ProtocolId,
};

pub type ProtocolMap = Rc<HashMap<ProtocolId, RcProtocol>>;

pub struct Machine {
    protocols: ProtocolMap,
    tap: Rc<RefCell<Tap>>,
}

impl Machine {
    pub fn new(tap: Rc<RefCell<Tap>>, protocols: impl Iterator<Item = RcProtocol>) -> Self {
        let tap_abstract: RcProtocol = tap.clone();
        let mut map = HashMap::new();
        for protocol in protocols.chain(std::iter::once(tap_abstract)) {
            let id = protocol.borrow().id();
            match map.entry(id) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    entry.insert(protocol);
                }
            }
        }
        Self {
            tap,
            protocols: Rc::new(map),
        }
    }

    pub fn awake(&mut self, context: &mut MachineContext) -> ControlFlow {
        let mut protocol_context = ProtocolContext::new(self.protocols.clone());
        for message in context.pending() {
            match self
                .tap
                .borrow_mut()
                // Todo: We want to get the network number from pending()
                .accept_incoming(message, 0, &mut protocol_context)
            {
                Ok(flow) => flow,
                Err(e) => {
                    eprintln!("{:?} -> {}", e, e);
                    continue;
                }
            }
        }

        let mut control_flow = ControlFlow::Continue;
        for protocol in self.protocols.values() {
            let flow = match protocol.borrow_mut().awake(&mut protocol_context) {
                Ok(flow) => flow,
                Err(e) => {
                    eprintln!("{:?} -> {}", e, e);
                    continue;
                }
            };
            match flow {
                ControlFlow::Continue => {}
                ControlFlow::EndSimulation => control_flow = ControlFlow::EndSimulation,
            }
        }

        let outgoing: HashMap<_, _> = self.tap.borrow_mut().outgoing().into_iter().collect();
        for (i, network) in context.networks().enumerate() {
            if let Some(messages) = outgoing.get(&(i as u8)) {
                for message in messages {
                    network
                        .borrow_mut()
                        // Todo: Use the correct physical address
                        .send(PhysicalAddress::Broadcast, message.clone());
                }
            }
        }

        control_flow
    }
}
