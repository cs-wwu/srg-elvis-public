use std::{any::TypeId, sync::Arc};

use elvis_core::{
    machine::ProtocolMap,
    protocols::{
        pci::pci_session::SessionInfo,
        user_process::{Application, ApplicationError},
        Pci, Udp, UserProcess,
    },
    Control, Message, Participants, Protocol, Shutdown,
};
use tokio::sync::Barrier;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QueryTester;

impl QueryTester {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

impl Application for QueryTester {
    fn start(
        &self,
        shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let slot_count = protocols
            .protocol::<Pci>()
            .expect("Missing PCI protocol")
            .slot_count();
        assert_eq!(slot_count, 2);

        let mut participants = Participants::new();
        participants.local.port = Some(0);
        participants.local.address = Some(0.into());
        participants.remote.port = Some(0);
        participants.remote.address = Some(0.into());
        let mtu = protocols
            .protocol::<Udp>()
            .expect("Missing UDP protocol")
            .open(TypeId::of::<UserProcess<Self>>(), participants, protocols)
            .unwrap()
            .info(TypeId::of::<Pci>())
            .expect("Missing PCI info")
            .downcast::<SessionInfo>()
            .expect("Downcast PCI session info")
            .mtu;
        assert_eq!(mtu, 1500);

        tokio::spawn(async move {
            initialize.wait().await;
            shutdown.shut_down();
        });
        Ok(())
    }

    fn receive(
        &self,
        _message: Message,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        unreachable!()
    }
}
