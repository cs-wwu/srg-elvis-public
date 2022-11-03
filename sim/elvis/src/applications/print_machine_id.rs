use elvis_core::{
    message::Message,
    protocol::{Context, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort},
        user_process::{Application, UserProcess},
        Udp, MACHINE_ID_KEY,
    },
    Control,
};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc::Sender, Barrier};

#[derive(Debug, Clone)]
pub struct PrintMachineId;

impl PrintMachineId {
    /// Creates a new capture.
    pub fn new() -> Self {
        Self
    }

    /// Creates a new capture behind a shared handle.
    pub fn new_shared() -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new())
    }
}

impl Application for PrintMachineId {
    const ID: ProtocolId = ProtocolId::from_string("Print Machine ID");

    fn start(
        self: Arc<Self>,
        context: Context,
        shutdown: Sender<()>,
        initialized: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>> {
        let mut participants = Control::new();
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0);
        RemotePort::set(&mut participants, 0);
        let session = context.protocol(Udp::ID).expect("No such protocol").open(
            Self::ID,
            participants,
            context,
        )?;
        println!("Machine ID: {:?}", session.query(MACHINE_ID_KEY));
        tokio::spawn(async move {
            initialized.wait().await;
            let _ = shutdown.send(()).await;
        });
        Ok(())
    }

    fn recv(self: Arc<Self>, _message: Message, _context: Context) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
