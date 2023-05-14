use super::Pci;
use crate::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Mac, Mtu},
    protocol::DemuxError,
    session::SendError,
    Control, Network, Session,
};
use std::{
    any::{Any, TypeId},
    sync::{Arc, RwLock},
};

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    mac: Mac,
    index: PciSlot,
    network: Arc<Network>,
    protocols: RwLock<Option<ProtocolMap>>,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(network: Arc<Network>, index: u32) -> Arc<Self> {
        let mac = network.next_mac();
        let this = Self {
            mac,
            index,
            network: network.clone(),
            protocols: Default::default(),
        };
        let this = Arc::new(this);
        network.register_tap(mac, this.clone());
        this
    }

    /// Called by the owning [`Pci`] protocol at the beginning of the simulation
    /// to start the contained tap running
    pub(super) fn start(&self, protocols: ProtocolMap) {
        *self.protocols.write().unwrap() = Some(protocols);
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) fn receive(self: &Arc<Self>, delivery: Delivery) -> Result<(), ReceiveError> {
        let mut control = Control::new();
        let protocols = self.protocols.read().unwrap().as_ref().unwrap().clone();
        control.slot = Some(self.index);
        control.remote.mac = Some(delivery.sender);
        let protocol = match protocols.get(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {0:?}",
                    delivery.protocol
                );
                Err(ReceiveError::Protocol(delivery.protocol))?
            }
        };
        protocol.demux(delivery.message, self.clone(), control, protocols)?;
        Ok(())
    }
}

impl Session for PciSession {
    #[tracing::instrument(name = "PciSession::send", skip_all)]
    fn send(
        &self,
        message: Message,
        control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), SendError> {
        let protocol = match control.first_responder {
            Some(protocol) => protocol,
            None => {
                tracing::error!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = control.remote.mac;

        if message.len() > self.network.mtu as usize {
            tracing::error!("Attempted to send a message larger than the network can handle");
            Err(SendError::Mtu(self.network.mtu))?
        }

        let delivery = Delivery {
            message,
            sender: self.mac,
            destination,
            protocol,
        };

        let network = self.network.clone();
        tokio::spawn(async move {
            network.send(delivery).await;
        });
        Ok(())
    }

    fn info(&self, protocol_id: TypeId) -> Option<Box<dyn Any>> {
        if protocol_id == TypeId::of::<Pci>() {
            Some(Box::new(SessionInfo {
                mac: self.mac,
                index: self.index,
                mtu: self.network.mtu,
            }))
        } else {
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Could not find a protocol for the given id: {0:?}")]
    Protocol(TypeId),
    #[error("{0}")]
    Demux(#[from] DemuxError),
}

pub struct SessionInfo {
    pub mac: Mac,
    pub index: PciSlot,
    pub mtu: Mtu,
}
