use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Tcp, MACHINE_ID_KEY, TAP_ID,
    },
    Control,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

#[derive(Debug, Clone)]
pub struct Query {
    id_recipient: Sender<u64>,
}

impl Query {
    /// Creates a new capture.
    pub fn new(id_recipient: Sender<u64>) -> Self {
        Self { id_recipient }
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared(id_recipient: Sender<u64>) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(id_recipient))
    }
}

impl Application for Query {
    const ID: ProtocolId = ProtocolId::from_string("Print Machine ID");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), ApplicationError> {
        let mut participants = Control::new();
        Ipv4::set_local_address(Ipv4Address::LOCALHOST, &mut participants);
        Ipv4::set_remote_address(Ipv4Address::LOCALHOST, &mut participants);
        Tcp::set_local_port(0, &mut participants);
        Tcp::set_remote_port(0, &mut participants);
        let session = context.protocol(Tcp::ID).expect("No such protocol").open(
            Self::ID,
            participants,
            context.clone(),
        )?;
        let tap = context.protocol(TAP_ID).expect("No such protocol");
        let machine_id_session = session.query(MACHINE_ID_KEY).unwrap().ok_u64().unwrap();
        let machine_id_protocol = tap.query(MACHINE_ID_KEY).unwrap().ok_u64().unwrap();
        assert_eq!(machine_id_session, machine_id_protocol);
        tokio::spawn(async move {
            initialized.wait().await;
            self.id_recipient.send(machine_id_session).await.unwrap();
            let _ = shutdown.send(()).await;
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
