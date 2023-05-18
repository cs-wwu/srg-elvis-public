use super::{ipv4_parsing::Ipv4HeaderBuilder, Ipv4, Ipv4Address, Recipient};
use crate::{
    machine::ProtocolMap,
    message::Message,
    protocol::DemuxError,
    protocols::{pci::PciSession, utility::Endpoints},
    session::SendError,
    Control, Session, Transport,
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
    pub(super) endpoints: AddressPair,
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

    pub fn endpoints(&self) -> AddressPair {
        self.endpoints
    }
}

impl Session for Ipv4Session {
    #[tracing::instrument(name = "Ipv4Session::send", skip_all)]
    fn send(&self, mut message: Message, _protocols: ProtocolMap) -> Result<(), SendError> {
        let length = message.iter().count();
        let transport: Transport = self.upstream.try_into().or(Err(SendError::Other))?;
        let header = match Ipv4HeaderBuilder::new(
            self.endpoints.local,
            self.endpoints.remote,
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
        message.header(header);
        self.pci_session
            .send_pci(message, self.recipient.mac, TypeId::of::<Ipv4>())?;
        Ok(())
    }
}

impl Debug for Ipv4Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ipv4Session")
            .field("identifier", &self.endpoints)
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
