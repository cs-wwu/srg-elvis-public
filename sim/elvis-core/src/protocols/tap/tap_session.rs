use super::{
    tap_misc::{FirstResponder, TapError},
    NetworkId,
};
use crate::{
    internet::NetworkHandle,
    machine::MachineId,
    message::Message,
    network::Delivery,
    protocol::{Context, ProtocolId},
    Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{error::Error, sync::Arc};
use tokio::sync::mpsc::Sender;

/// The session type for a [`Tap`](super::Tap).
#[derive(Clone)]
pub(crate) struct TapSession {
    /// The identifier for the machine this tap serves
    machine_id: MachineId,
    /// For now, we're just ignoring non-broadcast delivery options. If a
    /// message goes to a network, just send it to every machine on the network.
    /// It's inefficient, but we'll improve it when DHCP or something of the
    /// sort is incorporated.
    networks: DashMap<NetworkHandle, Sender<Delivery>>,
}

impl TapSession {
    /// Creates a new Tap session
    pub(super) fn new(machine_id: MachineId) -> Self {
        Self {
            machine_id,
            networks: Default::default(),
        }
    }

    /// Attaches this Tap to the given network
    pub fn attach(self: Arc<Self>, network_id: NetworkHandle, sender: Sender<Delivery>) {
        match self.networks.entry(network_id) {
            Entry::Occupied(_) => {
                panic!("Tried to attach same network twice");
            }
            Entry::Vacant(entry) => {
                entry.insert(sender);
            }
        }
    }

    /// Receives a delivery from the network and passes it up the protocol
    /// stack.
    pub(super) fn receive_delivery(
        self: Arc<Self>,
        mut delivery: Delivery,
        mut context: Context,
    ) -> Result<(), Box<dyn Error>> {
        let first_responder: FirstResponder = take_header(&delivery.message)
            .ok_or(TapError::HeaderLength)
            .unwrap()
            .into();
        first_responder.apply(&mut context.info);
        let network_id: NetworkId = delivery.network;
        network_id.apply(&mut context.info);
        delivery.message.slice(8..);
        let protocol = context
            .protocol(first_responder.into())
            .ok_or_else(|| TapError::NoSuchProtocol(first_responder.into()))?;
        protocol.demux(delivery.message, self, context)
    }
}

impl Session for TapSession {
    fn send(self: Arc<Self>, mut message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        let network_id = NetworkId::try_from(&context.info)?;
        let first_responder = FirstResponder::try_from(&context.info)?;
        message.prepend(first_responder.into_inner().to_be_bytes().to_vec());
        let delivery = Delivery {
            message,
            sender: self.machine_id,
            network: network_id,
        };
        tokio::spawn(async move {
            let sender = self
                .networks
                .get(&NetworkHandle::new(network_id.into_inner()))
                .unwrap()
                .clone();
            sender.send(delivery).await.unwrap()
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
}

/// Parses the Tap header from the message, which is just the ID of the protocol
/// that should receive this message.
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