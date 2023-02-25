use super::{ipv4_parsing::Ipv4HeaderBuilder, Ipv4, Ipv4Address};
use crate::{
    control::{Key, Primitive},
    id::Id,
    machine::PciSlot,
    message::Message,
    protocol::{Context, DemuxError, SharedProtocol},
    protocols::{pci::Pci, Arp},
    session::{QueryError, SendError, SharedSession},
    Network, Session, network::Mac, ProtocolMap,
};
use std::{fmt::Debug, sync::Arc};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    upstream: Id,
    /// The session we mux outgoing messages to
    downstream: SharedSession,
    /// The identifying information for this session
    id: SessionId,
    /// The PCI slot to send on
    tap_slot: PciSlot,
    /// The MAC address to send packets to
    remote_mac: Option<Mac>,
}

impl Ipv4Session {
    /// Creates a new IPv4 session
    pub(super) fn new(
        downstream: SharedSession,
        upstream: Id,
        identifier: SessionId,
        tap_slot: PciSlot,
        protocols: ProtocolMap,
    ) -> Self {
        // If we have an Arp instance,
        // Get the remote MAC address from the ARP.
        let arp = protocols.protocol(Arp::ID);
        let mut remote_mac = None;
        if let Some(arp) = arp {
            let remote_ip: u32 = identifier.remote.to_u32();
            // query arp for the remote mac
            let remote_mac_u64: u64 = arp.query((Arp::ID, remote_ip.into()))
            .expect("could not obtain MAC from arp")
            .try_into()
            .unwrap();
            remote_mac = Some(remote_mac_u64);
            // that was a doozy
        }
        Self {
            upstream,
            downstream,
            id: identifier,
            tap_slot,
            remote_mac,
        }
    }

    pub fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), DemuxError> {
        context
            .protocol(self.upstream)
            .expect("No such protocol")
            .demux(message, self, context)?;
        Ok(())
    }
}

impl Session for Ipv4Session {
    #[tracing::instrument(name = "Ipv4Session::send", skip(message, context))]
    fn send(self: Arc<Self>, mut message: Message, mut context: Context) -> Result<(), SendError> {
        let length = message.iter().count();
        let header = match Ipv4HeaderBuilder::new(
            self.id.local,
            self.id.remote,
            self.upstream.into_inner() as u8,
            length as u16,
        )
        .build()
        {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(SendError::Header)?
            }
        };

        if let Some(remote_mac_addr) = self.remote_mac {
            Network::set_destination(remote_mac_addr, &mut context.control);
        }

        Pci::set_pci_slot(self.tap_slot, &mut context.control);
        Network::set_protocol(Ipv4::ID, &mut context.control);
        message.header(header);
        self.downstream.clone().send(message, context)?;
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}

impl Debug for Ipv4Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ipv4Session")
            .field("identifier", &self.id)
            .finish()
    }
}

/// A set that uniquely identifies a given session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SessionId {
    /// The local address
    pub local: Ipv4Address,
    /// The remote address
    pub remote: Ipv4Address,
}

impl SessionId {
    pub fn new(local: Ipv4Address, remote: Ipv4Address) -> Self {
        Self { local, remote }
    }
}
