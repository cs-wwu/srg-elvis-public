use crate::{
    protocol::{Context, ProtocolId},
    session::SharedSession,
    Control, Message, Protocol,
};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc::Sender, Barrier};

mod stolen;

pub struct Tcp {}

impl Tcp {
    pub const ID: ProtocolId = ProtocolId::new(6);
}

impl Protocol for Tcp {
    fn id(self: Arc<Self>) -> ProtocolId {
        todo!()
    }

    fn open(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        todo!()
    }

    fn listen(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn demux(
        self: Arc<Self>,
        _message: Message,
        _caller: SharedSession,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn start(
        self: Arc<Self>,
        _context: Context,
        _shutdown: Sender<()>,
        _initialized: Arc<Barrier>,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
