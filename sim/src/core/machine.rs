use super::{MachineContext, Protocol, ProtocolId, Session, SessionId};
use std::collections::HashMap;

pub struct Machine {
    /// The first protocol should be the first to receive messages
    protocols: Vec<ProtocolConnections>,
    sessions: HashMap<SessionId, SessionConnections>,
}

impl Machine {
    pub fn new(protocol_configs: Vec<ProtocolConfig>) -> Self {
        let parents: Vec<_> = protocol_configs
            .iter()
            .enumerate()
            .map(|(child_i, _)| {
                let parents: Vec<_> = protocol_configs
                    .iter()
                    .enumerate()
                    .filter_map(|(parent_i, config)| {
                        if config.children.contains(&child_i) {
                            Some(parent_i)
                        } else {
                            None
                        }
                    })
                    .collect();
                parents
            })
            .collect();

        let protocols: Vec<_> = protocol_configs
            .into_iter()
            .zip(parents.into_iter())
            .map(|(config, parents)| ProtocolConnections {
                protocol: config.protocol,
                parents,
                children: config.children,
            })
            .collect();

        Self {
            protocols,
            sessions: Default::default(),
        }
    }

    pub fn awake(&mut self, context: &mut MachineContext) {
        self.receive_pending(context);
    }

    fn receive_pending(&mut self, context: &mut MachineContext) {
        // for message in context.pending() {
        //     let mut responder = self.protocols.first();
        //     while let Some(connections) = responder {
        //         match connections.protocol.demux(message.clone()) {
        //             Ok(session_id) => {
        //                 let protocol_id = match self
        //                     .sessions
        //                     .get(&session_id)
        //                     .expect("No session for session ID")
        //                     .session
        //                     .recv(message.clone())
        //                 {
        //                     Ok(protocol_id) => Some(protocol_id),
        //                     Err(err) => {
        //                         eprintln!("{}", err);
        //                         None
        //                     }
        //                 };

        //                 if let Some(protocol_id) = protocol_id {
        //                     responder = self.protocols.get(protocol_id);
        //                 } else {
        //                     eprintln!("No protocol for protocol ID")
        //                 }
        //             }
        //             Err(e) => eprintln!("{}", e),
        //         }
        //     }
        // }
        todo!()
    }
}

pub struct ProtocolConfig {
    protocol: Box<dyn Protocol>,
    children: Vec<ProtocolId>,
}

struct ProtocolConnections {
    protocol: Box<dyn Protocol>,
    parents: Vec<ProtocolId>,
    children: Vec<ProtocolId>,
}

struct SessionConnections {
    session: Box<dyn Session>,
    parents: Vec<SessionId>,
    children: Vec<SessionId>,
}
