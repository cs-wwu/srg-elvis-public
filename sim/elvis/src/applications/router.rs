use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::{AddressPair, Ipv4Address},
        user_process::{Application, ApplicationError},
        Ipv4, UserProcess,
    },
    Control, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

impl Application for Router {
    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let ipv4 = protocols.protocol::<Ipv4>().expect("Router requires IPv4");
        ipv4.listen(
            TypeId::of::<UserProcess<Self>>(),
            Ipv4Address::CURRENT_NETWORK,
        )
        .unwrap();
        tokio::spawn(async move {
            initialize.wait().await;
        });
        Ok(())
    }

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn receive(
        &self,
        message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        println!("Hit: {control:?}");
        let ipv4 = protocols.protocol::<Ipv4>().expect("Router requires IPv4");
        let addresses = *control.get::<AddressPair>().unwrap();
        let session = ipv4
            .open(
                TypeId::of::<UserProcess<Self>>(),
                addresses,
                protocols.clone(),
            )
            .unwrap();
        session.send(message, protocols)?;
        Ok(())
    }
}
