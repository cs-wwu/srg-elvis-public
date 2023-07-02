use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, Recipients},
        Ipv4, Pci,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Router {
    ip_table: Recipients,
}

impl Router {
    pub fn new(ip_table: Recipients) -> Self {
        Self { ip_table }
    }
}

#[async_trait::async_trait]
impl Protocol for Router {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let ipv4 = protocols.protocol::<Ipv4>().expect("Router requires IPv4");
        ipv4.listen(self.id(), Ipv4Address::CURRENT_NETWORK, protocols)
            .unwrap();
        initialize.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        mut message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let mut ipv4_header = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;
        ipv4_header.time_to_live -= 1;
        if ipv4_header.time_to_live == 0 {
            return Ok(());
        }
        // TODO(hardint): Fragmentation
        message.header(ipv4_header.serialize().or(Err(DemuxError::Other))?);
        let recipient = self
            .ip_table
            .get(&ipv4_header.destination)
            .ok_or(DemuxError::Other)?;
        let session = protocols.protocol::<Pci>().unwrap().open(recipient.slot);
        session.send_pci(message, recipient.mac, TypeId::of::<Ipv4>())?;
        Ok(())
    }
}
