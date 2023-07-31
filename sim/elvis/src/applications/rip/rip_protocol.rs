use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, Recipient},
        Ipv4, Pci,
    },
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

use crate::applications::ArpRouter;

// number of seconds between each update
const UPDATE: u32 = 30;
pub struct Rip {
    inner: ArpRouter
}

impl Rip {
    pub async fn update() {}
}

#[async_trait::async_trait]
impl Protocol for Rip {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        let _ = self.inner.start(shutdown, initialized, protocols);
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        todo!()
    }
}
