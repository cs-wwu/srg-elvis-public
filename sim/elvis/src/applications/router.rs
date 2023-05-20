use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, Recipients},
        user_process::{Application, ApplicationError},
        Ipv4, Pci, UserProcess,
    },
    Control, Shutdown, Transport,
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
        mut message: Message,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        let mut ipv4_header = control
            .get::<Ipv4Header>()
            .ok_or(ApplicationError::Other)?
            .clone();
        ipv4_header.time_to_live -= 1;
        if ipv4_header.time_to_live == 0 {
            return Ok(());
        }
        // TODO(hardint): Fragmentation
        message.header(ipv4_header.serialize().or(Err(ApplicationError::Other))?);
        let recipient = self
            .ip_table
            .get(&ipv4_header.destination)
            .ok_or(ApplicationError::Other)?;
        let session = protocols.protocol::<Pci>().unwrap().open(recipient.slot);
        session.send_pci(message, recipient.mac, TypeId::of::<Ipv4>())?;
        Ok(())
    }
}
