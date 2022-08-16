use super::{
    tap_misc::{Delivery, FirstResponder, TapError},
    NetworkId,
};
use crate::core::{
    internet::{NetworkHandle, NetworkInfo},
    machine::MachineId,
    message::Message,
    protocol::{Context, ProtocolId},
    Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};

#[derive(Clone)]
pub(crate) struct TapSession {
    machine_id: MachineId,
    /// For now, we're just ignoring non-broadcast delivery options. If a
    /// message goes to a network, just send it to every machine on the network.
    /// It's inefficient, but we'll improve it when DHCP or something of the
    /// sort is incorporated.
    networks: DashMap<NetworkHandle, Arc<NetworkInfo>>,
}

impl TapSession {
    pub(super) fn new(machine_id: MachineId) -> Self {
        Self {
            machine_id,
            networks: Default::default(),
        }
    }

    pub fn attach(self: Arc<Self>, network_id: NetworkHandle, network_info: Arc<NetworkInfo>) {
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
        mut context: Context,
    ) -> Result<(), Box<dyn Error>> {
        let first_responder: FirstResponder = take_header(&delivery.message)
            .ok_or(TapError::HeaderLength)
            .unwrap()
            .into();
        first_responder.apply(&mut context.info);
        let network_id: NetworkId = delivery.network;
        network_id.apply(&mut context.info);
        let message = delivery.message.slice(8..);
        let protocol = context
            .protocol(first_responder.into())
            .ok_or_else(|| TapError::NoSuchProtocol(first_responder.into()))?;
        protocol.demux(message, self, context)
    }
}

impl Session for TapSession {
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        let network_id = NetworkId::try_from(&context.info)?;
        let first_responder = FirstResponder::try_from(&context.info)?;
        let message = message.with_header(first_responder.into_inner().to_be_bytes().to_vec());
        let delivery = Delivery {
            message,
            sender: self.machine_id,
            network: network_id,
        };
        println!("Tap session");
        tokio::spawn(async move {
            let network = self
                .networks
                .get(&NetworkHandle::new(network_id.into_inner()))
                .unwrap();
            for sender in network.senders.iter().filter_map(|(machine_id, sender)| {
                (*machine_id != self.machine_id).then_some(sender)
            }) {
                sender.send(delivery.clone()).await.unwrap();
            }
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        panic!("Use Tap::receive_delivery() instead");
    }

    fn start(self: Arc<Self>, _context: Context) -> Result<(), Box<dyn Error>> {
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
