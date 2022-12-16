use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort},
        user_process::{Application, ApplicationError, UserProcess},
        Udp, MACHINE_ID_KEY, TAP_ID,
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
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0);
        RemotePort::set(&mut participants, 0);
        let session = context.protocol(Udp::ID).expect("No such protocol").open(
            Self::ID,
            participants,
            context.clone(),
        )?;
        let tap = context.protocol(TAP_ID).expect("No such protocol");
        let machine_id_session = match session.query(MACHINE_ID_KEY).unwrap().ok_u64() {
            Ok(machine_id_session) => machine_id_session,
            Err(e) => {
                tracing::error!("{}", e);
                Err(ApplicationError::Other)?
            }
        };
        let machine_id_protocol = match tap.query(MACHINE_ID_KEY).unwrap().ok_u64() {
            Ok(machine_id_protocol) => machine_id_protocol,
            Err(e) => {
                tracing::error!("{}", e);
                Err(ApplicationError::Other)?
            }
        };
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
