use std::sync::Arc;

use elvis_core::{
    machine::ProtocolMap,
    protocols::{
        user_process::{Application, ApplicationError},
        Pci, Udp, UserProcess,
    },
    Control, Id, Message, Shutdown,
};
use tokio::sync::Barrier;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QueryTester;

impl QueryTester {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for QueryTester {
    const ID: Id = Id::from_string("Query tester");

    fn start(
        &self,
        shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let slot_count = protocols
            .protocol(Pci::ID)
            .expect("Missing PCI protocol")
            .query(Pci::SLOT_COUNT_QUERY_KEY)
            .unwrap()
            .ok_u64()
            .unwrap();
        assert_eq!(slot_count, 2);

        let mut participants = Control::new();
        participants.local.port = Some(0);
        participants.local.address = Some(0.into());
        participants.remote.port = Some(0);
        participants.remote.address = Some(0.into());
        let mtu = protocols
            .protocol(Udp::ID)
            .expect("Missing UDP protocol")
            .open(Self::ID, participants, protocols)
            .unwrap()
            .query(Pci::MTU_QUERY_KEY)
            .unwrap()
            .ok_u32()
            .unwrap();
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
