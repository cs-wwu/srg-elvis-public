use std::sync::Arc;

use elvis_core::{
    networks::Generic,
    protocol::Context,
    protocols::{
        user_process::{Application, ApplicationError},
        Ipv4, Pci, Udp, UserProcess,
    },
    Control, Id, Message,
};
use tokio::sync::{mpsc::Sender, Barrier};

pub struct QueryTester;

impl QueryTester {
    pub fn new() -> Self {
        Self
    }

    pub fn new_shared() -> Arc<UserProcess<Self>> {
        Arc::new(UserProcess::new(Self::new()))
    }
}

impl Application for QueryTester {
    const ID: Id = Id::from_string("Query tester");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialize: Arc<Barrier>,
    ) -> Result<(), ApplicationError> {
        let slot_count = context
            .protocol(Pci::ID)
            .expect("Missing PCI protocol")
            .query(Pci::SLOT_COUNT_QUERY_KEY)
            .unwrap()
            .ok_u64()
            .unwrap();
        assert_eq!(slot_count, 2);

        let mut participants = Control::new();
        Udp::set_local_port(0, &mut participants);
        Udp::set_remote_port(0, &mut participants);
        Ipv4::set_local_address(0.into(), &mut participants);
        Ipv4::set_remote_address(0.into(), &mut participants);
        let mtu = context
            .protocol(Udp::ID)
            .expect("Missing UDP protocol")
            .open(Self::ID, participants, context)
            .unwrap()
            .query(Generic::MTU_QUERY_KEY)
            .unwrap()
            .ok_u32()
            .unwrap();
        assert_eq!(mtu, 1500);

        tokio::spawn(async move {
            initialize.wait().await;
            let _ = shutdown.send(()).await;
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        unreachable!()
    }
}
