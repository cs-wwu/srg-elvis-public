use crate::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Mac, Mtu},
    protocol::DemuxError,
    session::SendError,
    Control, Network, Session,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use super::DemuxInfo;

/// The session type associated with [`Pci`](super::Pci).
/// Contains information about a connection between this MAC address and the network.
pub struct PciSession {
    mac: Mac,
    slot: PciSlot,
    network: Arc<Network>,
    protocols: RwLock<Option<ProtocolMap>>,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(network: Arc<Network>, index: u32) -> Arc<Self> {
        let mac = network.next_mac();
        let this = Self {
            mac,
            slot: index,
            network: network.clone(),
            protocols: Default::default(),
        };
        let this = Arc::new(this);
        network.register_tap(mac, this.clone());
        this
    }

    pub fn mac(&self) -> Mac {
        self.mac
    }

    pub fn slot(&self) -> PciSlot {
        self.slot
    }

    pub fn mtu(&self) -> Mtu {
        self.network.mtu
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
        let pci_demux_info = DemuxInfo {
            slot: self.slot,
            source: delivery.sender,
            destination: delivery.destination,
            mtu: self.network.mtu,
        };
        control.insert(pci_demux_info);
        let protocols = self.protocols.read().unwrap().as_ref().unwrap().clone();
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

    pub fn send_pci(
        &self,
        message: Message,
        remote_mac: Option<Mac>,
        receiver: TypeId,
    ) -> Result<(), SendError> {
        if message.len() > self.network.mtu as usize {
            return Err(SendError::Mtu(self.network.mtu));
        }

        let delivery = Delivery {
            message,
            sender: self.mac,
            destination: remote_mac,
            protocol: receiver,
        };

        let network = self.network.clone();
        tokio::spawn(async move {
            network.send(delivery).await;
        });
        Ok(())
    }
}

// TODO(hardint): I think PCI sessions maybe just shouldn't implement to session type
impl Session for PciSession {
    fn send(&self, _message: Message, _protocols: ProtocolMap) -> Result<(), SendError> {
        panic!("Should use PciSession::send_pci instead");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Could not find a protocol for the given id: {0:?}")]
    Protocol(TypeId),
    #[error("{0}")]
    Demux(#[from] DemuxError),
}
