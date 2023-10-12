use super::{
    fragmentation,
    ipv4_parsing::{Ipv4Header, Ipv4HeaderBuilder},
    Ipv4, Ipv4Address, Recipient,
};
use crate::{
    machine::ProtocolMap,
    message::Message,
    network::Delivery,
    protocol::DemuxError,
    protocols::{pci::PciSession, utility::Endpoints},
    session::SendError,
    Control, Network, Session, Transport,
};
use std::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

/// The session type for [`Ipv4`].
pub struct Ipv4Session {
    /// The protocol that we demux incoming messages to
    pub(super) upstream: TypeId,
    /// The session we mux outgoing messages to
    pub(super) pci_session: Arc<PciSession>,
    /// The identifying information for this session
    pub(super) addresses: AddressPair,
    /// Information about how and where to send packets
    pub(super) recipient: Recipient,
}

impl Ipv4Session {
    pub fn receive(
        self: Arc<Self>,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        protocols
            .get(self.upstream)
            .expect("No such protocol")
            .demux(message, self, control, protocols)?;
        Ok(())
    }

    pub fn pci_session(&self) -> &PciSession {
        self.pci_session.as_ref()
    }

    pub fn addresses(&self) -> AddressPair {
        self.addresses
    }

    // Sends a single message. Precondition: message is small enough to fit the mtu.
    #[tracing::instrument(name = "Ipv4Session::send_smol", skip_all)]
    fn send_smol(&self, headmsg: Message) -> Result<(), SendError> {
        if self.addresses.remote == Ipv4Address::SUBNET {
            self.pci_session.send_pci(
                headmsg,
                Some(Network::BROADCAST_MAC),
                TypeId::of::<Ipv4>(),
            )?;
        } else if crate::subnetting::Ipv4Net::LOOPBACK.contains(self.addresses.remote) {
            // address is a loopback address (127.0.0.1/8),
            // so we send the things directly to our own tap slot.
            let delivery = Delivery {
                message: headmsg,
                sender: self.pci_session.mac(),
                destination: Some(self.pci_session.mac()),
                protocol: TypeId::of::<Ipv4>(),
            };
            match self.pci_session.receive(delivery) {
                Ok(_) => {}
                Err(_) => {
                    tracing::error!(
                        "Failed to send to loopback address {:?}",
                        self.addresses.remote
                    );
                    return Err(SendError::Other);
                }
            }
        } else {
            self.pci_session
                .send_pci(headmsg, self.recipient.mac, TypeId::of::<Ipv4>())?;
        }

        Ok(())
    }
}

impl Session for Ipv4Session {
    #[tracing::instrument(name = "Ipv4Session::send", skip_all)]
    fn send(&self, mut message: Message, _protocols: ProtocolMap) -> Result<(), SendError> {
        let length = message.len();
        let transport: Transport = self.upstream.try_into().or(Err(SendError::Other))?;
        let header = match Ipv4HeaderBuilder::new(
            self.addresses.local,
            self.addresses.remote,
            transport as u8,
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

        // fragment header and send frags if necessary
        let mtu = self.pci_session().mtu();
        if message.len() + header.len() > mtu as usize {
            // due to limitations in how the Ipv4Header and Builder work,
            // I need to deserialize the header
            let header_de = Ipv4Header::from_bytes(header.iter().copied())
                .expect("we should be able to deserialize our own header");
            match fragmentation::fragment(header_de, message, mtu) {
                fragmentation::Fragments::Discard => return Ok(()),
                fragmentation::Fragments::DontFragment(_) => unreachable!(),
                fragmentation::Fragments::Fragmented(pairs) => {
                    for (header, mut message) in pairs {
                        let header_bytes =
                            header.serialize().expect("header shouldn't have errors");
                        message.header(header_bytes);
                        self.send_smol(message)?;
                    }
                    return Ok(());
                }
            }
        }

        // if fragmentation was not necessary
        message.header(header);
        self.send_smol(message)
    }
}

impl Debug for Ipv4Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ipv4Session")
            .field("addresses", &self.addresses)
            .finish()
    }
}

/// A set that uniquely identifies a given session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressPair {
    /// The local address
    pub local: Ipv4Address,
    /// The remote address
    pub remote: Ipv4Address,
}

impl From<Endpoints> for AddressPair {
    fn from(value: Endpoints) -> Self {
        Self {
            local: value.local.address,
            remote: value.remote.address,
        }
    }
}
