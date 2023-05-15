use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError},
        Ipv4, UserProcess,
    },
    Control, Participants, Protocol, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self
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
        let mut participants = Participants::new();
        participants.local.address = Some(Ipv4Address::CURRENT_NETWORK);
        ipv4.listen(TypeId::of::<UserProcess<Self>>(), participants, protocols)?;
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
        let mut participants = Participants::new();
        participants.local.address = control.local.address;
        participants.remote.address = control.remote.address;
        let session = ipv4.open(
            TypeId::of::<UserProcess<Self>>(),
            participants,
            protocols.clone(),
        )?;
        session.send(message, Control::new(), protocols)?;
        Ok(())
    }
}
