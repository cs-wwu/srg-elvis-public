//! An implementation of [Internet Protocol version
//! 4](https://datatracker.ietf.org/doc/html/rfc791).

use crate::{
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    network::Mac,
    protocol::{DemuxError, StartError},
    protocols::pci::Pci,
    Control, FxDashMap, IpTable, Protocol, Session, Shutdown,
};
use dashmap::mapref::entry::Entry;
use rustc_hash::FxHashMap;
use std::{any::TypeId, sync::{Arc, RwLock}};
use tokio::sync::Barrier;

pub mod ipv4_parsing;
use ipv4_parsing::Ipv4Header;

mod ipv4_address;
pub use ipv4_address::Ipv4Address;

pub(crate) mod ipv4_session;
pub use ipv4_session::AddressPair;
use ipv4_session::Ipv4Session;

use super::{pci, Arp, arp::subnetting::Ipv4Mask};

pub mod fragmentation;
mod reassembly;
mod test_header_builder;

/// An implementation of the Internet Protocol.
pub struct Ipv4 {
    listen_bindings: FxDashMap<Ipv4Address, TypeId>,
    recipients: IpTable<Recipient>,
    info: Vec<RwLock<Ipv4Info>>,
}

impl Ipv4 {
    /// Creates a new instance of the protocol.
    pub fn new(recipients: IpTable<Recipient>) -> Self {
        Self {
            listen_bindings: Default::default(),
            recipients,
            info: Default::default(),
        }
    }

    pub async fn open_and_listen(
        &self,
        upstream: TypeId,
        endpoints: AddressPair,
        protocols: ProtocolMap,
    ) -> Result<Arc<Ipv4Session>, OpenAndListenError> {
        self.listen(upstream, endpoints.local, protocols.clone())?;
        Ok(self
            .open_for_sending(upstream, endpoints, protocols)
            .await?)
    }

    pub async fn open_for_sending(
        &self,
        upstream: TypeId,
        endpoints: AddressPair,
        protocols: ProtocolMap,
    ) -> Result<Arc<Ipv4Session>, OpenError> {
        let mut recipient = match self.recipients.get_recipient(endpoints.remote) {
            Some(recipient) => recipient,
            None => {
                return Err(OpenError::UnknownRecipient(endpoints.remote));
            }
        };

        // if ARP exists, and recipient does not specify a destination MAC, then try to figure out a destination MAC
        if recipient.mac.is_none() {
            if let Some(arp) = protocols.protocol::<Arp>() {
                arp.listen(endpoints.local);
                let resolved_mac = arp
                    .resolve(endpoints, recipient.slot, protocols.clone())
                    .await?;
                recipient.mac = Some(resolved_mac);
            }
        }

        let pci_session = protocols.protocol::<Pci>().unwrap().open(recipient.slot);
        let session = Arc::new(Ipv4Session {
            pci_session,
            upstream,
            reassembly: Default::default(),
            addresses: endpoints,
            recipient,
        });
        Ok(session)
    }

    pub fn listen(
        &self,
        upstream: TypeId,
        address: Ipv4Address,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        if let Some(arp) = protocols.protocol::<Arp>() {
            arp.listen(address);
        }
        match self.listen_bindings.entry(address) {
            Entry::Occupied(entry) => {
                if *entry.get() == upstream {
                    entry.replace_entry(upstream);
                    Ok(())
                } else {
                    Err(ListenError::Exists(address))
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(upstream);
                Ok(())
            }
        }
    }
}

// TODO(hardint): Add a static IP lookup table in the constructor so that
// messages can be sent to the correct network
#[async_trait::async_trait]
impl Protocol for Ipv4 {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        _protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        mut message: Message,
        _caller: Arc<dyn Session>,
        mut control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Extract identifying information from the header and the context and
        // add header information to the context
        let header = match Ipv4Header::from_bytes(message.iter()) {
            Ok(header) => header,
            Err(e) => {
                tracing::error!("{}", e);
                Err(DemuxError::Header)?
            }
        };
        control.insert(header);
        message.remove_front(header.ihl as usize * 4);
        let endpoints = AddressPair {
            local: header.destination,
            remote: header.source,
        };
        // If the session does not exist, see if we have a listen
        // binding for it
        let upstream = match self.listen_bindings.get(&endpoints.local) {
            Some(binding) => *binding,

            None => {
                // If we don't have a normal listen binding, check for
                // a 0.0.0.0 binding
                match self.listen_bindings.get(&Ipv4Address::CURRENT_NETWORK) {
                    Some(binding) => *binding,
                    None => {
                        tracing::error!(
                            "Could not find a listen binding for the local address {}",
                            endpoints.local
                        );
                        Err(DemuxError::MissingSession)?
                    }
                }
            }
        };

        let pci_demux_info = control
            .get::<pci::DemuxInfo>()
            .ok_or(DemuxError::MissingContext)?;
        let recipient = Recipient::with_mac(pci_demux_info.slot, pci_demux_info.source);
        let session = Arc::new(Ipv4Session {
            upstream,
            pci_session: protocols
                .protocol::<Pci>()
                .unwrap()
                .open(pci_demux_info.slot),
            addresses: AddressPair {
                local: header.destination,
                remote: header.source,
            },
            recipient,
            reassembly: Default::default(),
        });
        session.receive(header, message, control, protocols)?;
        Ok(())
    }
}

pub type Recipients = FxHashMap<Ipv4Address, Recipient>;

// TODO(hardint): Rename to something like pci::SendInfo and move to the PCI module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Recipient {
    pub slot: PciSlot,
    pub mac: Option<Mac>,
}

impl Recipient {
    pub fn new(slot: PciSlot, mac: Option<Mac>) -> Self {
        Self { slot, mac }
    }

    pub fn with_mac(slot: PciSlot, mac: Mac) -> Self {
        Self::new(slot, Some(mac))
    }

    pub fn broadcast(slot: PciSlot) -> Self {
        Self::new(slot, None)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Ipv4Info {
    // The slot this struct maps to
    pub tap_slot: PciSlot,
    // configuration options
    pub ip_address: Option<Ipv4Address>,
    pub subnet_mask: Option<Ipv4Mask>,
    pub default_gateway: Option<Ipv4Address>,
    pub dns_server: Option<Ipv4Address>,
}

impl Ipv4Info {
    pub fn new(tap_slot: PciSlot) -> Self {
        Self {
            tap_slot,
            ip_address: None,
            subnet_mask: None,
            default_gateway: None,
            dns_server: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum OpenError {
    #[error("The IP table is missing an entry for {0}")]
    UnknownRecipient(Ipv4Address),
    #[error("Arp was unable to resolve MAC address")]
    ArpFailure(#[from] crate::protocols::arp::NoResponseError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ListenError {
    #[error("There is already a session for {0:?}")]
    Exists(Ipv4Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenAndListenError {
    #[error("{0}")]
    Open(#[from] OpenError),
    #[error("{0}")]
    Listen(#[from] ListenError),
}
