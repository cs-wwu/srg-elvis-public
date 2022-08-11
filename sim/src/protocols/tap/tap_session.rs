use super::{
    tap_misc::{Delivery, FirstResponder, TapError},
    NetworkId,
};
use crate::core::{message::Message, MachineId, NetworkInfo, ProtocolContext, ProtocolId, Session};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};

#[derive(Clone)]
pub struct TapSession {
    machine_id: MachineId,
    /// For now, we're just ignoring non-broadcast delivery options. If a
    /// message goes to a network, just send it to every machine on the network.
    /// It's inefficient, but we'll improve it when DHCP or something of the
    /// sort is incorporated.
    networks: DashMap<crate::core::NetworkId, Arc<NetworkInfo>>,
}

impl TapSession {
    pub(super) fn new(machine_id: MachineId) -> Self {
        Self {
            machine_id,
            networks: Default::default(),
        }
    }

    pub fn attach(
        self: Arc<Self>,
        network_id: crate::core::NetworkId,
        network_info: Arc<NetworkInfo>,
    ) {
        match self.networks.entry(network_id) {
            Entry::Occupied(_) => {
                panic!("Tried to attach same network twice");
            }
            Entry::Vacant(entry) => {
                entry.insert(network_info);
            }
        }
    }

    pub(super) fn receive_delivery(
        self: Arc<Self>,
        delivery: Delivery,
        mut context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let first_responder: FirstResponder = take_header(&delivery.message)
            .ok_or(TapError::HeaderLength)
            .unwrap()
            .into();
        first_responder.apply(&mut context.info);
        let network_id: NetworkId = delivery.network.into();
        network_id.apply(&mut context.info);
        let message = delivery.message.slice(8..);
        let protocol = context.protocol(first_responder.into()).unwrap();
        protocol.demux(message, self, context)
    }
}

impl Session for TapSession {
    fn send(
        self: Arc<Self>,
        message: Message,
        context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        let network_id = NetworkId::try_from(&context.info)?;
        let first_responder = FirstResponder::try_from(&context.info)?;
        let message = message.with_header(first_responder.into_inner().to_be_bytes().to_vec());
        let delivery = Delivery {
            message: message,
            sender: self.machine_id,
            network: network_id,
        };
        tokio::spawn(async move {
            let network = self.networks.get(&network_id.into_inner()).unwrap();
            for sender in network.senders.iter() {
                sender.send(delivery.clone()).await.unwrap();
            }
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Use Tap::receive_delivery() instead");
    }

    fn start(self: Arc<Self>, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

fn take_header(message: &Message) -> Option<ProtocolId> {
    let mut iter = message.iter();
    Some(
        u64::from_be_bytes([
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
        ])
        .into(),
    )
}
