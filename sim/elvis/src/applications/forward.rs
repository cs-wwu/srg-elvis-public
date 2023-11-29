use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{udp::Udp, Endpoints},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
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
}

#[async_trait::async_trait]
impl Protocol for Forward {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        let udp = machine.protocol::<Udp>().expect("No such protocol");
        *self.outgoing.write().unwrap() = Some(
            udp.open_and_listen(
                self.id(),
                // TODO(hardint): Can these clones be cheaper?
                self.endpoints,
                machine,
            )
            .await
            .unwrap(),
        );
        initialized.wait().await;

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        self.outgoing
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(message, machine)?;
        Ok(())
    }
}
