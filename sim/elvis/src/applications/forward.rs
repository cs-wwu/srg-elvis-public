use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        udp::Udp,
        user_process::{Application, ApplicationError, UserProcess},
        Endpoints,
    },
    Control, Session, Shutdown,
};
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

/// An application that forwards messages to `local_ip` to `remote_ip`.
pub struct Forward {
    /// The session on which we send any messages we receive
    outgoing: RwLock<Option<Arc<dyn Session>>>,
    endpoints: Endpoints,
}

impl Forward {
    /// Creates a new forwarding application.
    pub fn new(endpoints: Endpoints) -> Self {
        Self {
            outgoing: Default::default(),
            endpoints,
        }
    }

    /// Creates a new forwarding application behind a shared handle.
    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

#[async_trait::async_trait]
impl Application for Forward {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let udp = protocols.protocol::<Udp>().expect("No such protocol");
        *self.outgoing.write().unwrap() = Some(
            udp.open_and_listen(
                TypeId::of::<UserProcess<Self>>(),
                // TODO(hardint): Can these clones be cheaper?
                self.endpoints,
                protocols,
            )
            .await
            .unwrap(),
        );
        initialized.wait().await;

        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        self.outgoing
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(message, protocols)?;
        Ok(())
    }
}
