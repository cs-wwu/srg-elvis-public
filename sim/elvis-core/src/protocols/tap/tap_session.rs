use super::{tap_misc::FirstResponder, NetworkId, MACHINE_ID_KEY};
use crate::{
    control::{Key, Primitive},
    internet::NetworkHandle,
    machine::MachineId,
    message::Message,
    network::Delivery,
    protocol::{Context, ProtocolId},
    session::{ReceiveError, SendError},
    Session,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{fmt::Debug, sync::Arc};
use tokio::sync::mpsc::Sender;

/// The session type for a [`Tap`](super::Tap).
#[derive(Clone)]
pub(crate) struct TapSession {
    /// The identifier for the machine this tap serves
    pub machine_id: MachineId,
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
    #[tracing::instrument(name = "TapSession::receive_delivery", skip(delivery, context))]
    pub(super) fn receive_delivery(
        self: Arc<Self>,
        mut delivery: Delivery,
        mut context: Context,
    ) -> Result<(), ReceiveError> {
        let first_responder: FirstResponder = take_header(&delivery.message)
            .expect("Expected two bytes for the header")
            .into();
        first_responder.apply(&mut context.info);
        let network_id: NetworkId = delivery.network;
        network_id.apply(&mut context.info);
        delivery.message.slice(8..);
        let protocol = match context.protocol(first_responder.into()) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID: {}",
                    first_responder
                );
                Err(ReceiveError::Other)?
            }
        };
        protocol.demux(delivery.message, self, context)?;
        Ok(())
    }
}

impl Session for TapSession {
    #[tracing::instrument(name = "TapSession::send", skip(message, context))]
    fn send(self: Arc<Self>, mut message: Message, context: Context) -> Result<(), SendError> {
        let network_id = match NetworkId::try_from(&context.info) {
            Ok(network_id) => network_id,
            Err(_) => {
                tracing::error!("The context did not contain a network ID");
                Err(SendError::MissingContext)?
            }
        };
        let first_responder = match FirstResponder::try_from(&context.info) {
            Ok(first_responder) => first_responder,
            Err(_) => {
                tracing::error!("The context did not contain a first responder");
                Err(SendError::MissingContext)?
            }
        };
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

    fn receive(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), ReceiveError> {
        panic!("Use Tap::receive_delivery() instead");
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, ()> {
        // TODO(hardint): Add support for querying the MTU
        match key {
            MACHINE_ID_KEY => Ok(self.machine_id.into()),
            _ => Err(()),
        }
    }
}

impl Debug for TapSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TapSession")
            .field("machine_id", &self.machine_id)
            .finish()
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
