//! The base-level protocol that communicates directly with networks.

use crate::{
    internet::NetworkHandle,
    machine::MachineId,
    message::Message,
    network::Delivery,
    protocol::{Context, ProtocolId},
    session::SharedSession,
    Control, Protocol, Session,
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{self, Receiver, Sender};

mod tap_misc;
pub use tap_misc::*;

mod tap_session;
use tap_session::TapSession;

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
pub(crate) struct Tap {
    receiver: Arc<Mutex<Option<Receiver<Delivery>>>>,
    session: Arc<TapSession>,
}

impl Tap {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::from_string("Tap");

    /// Creates a new network tap.
    pub fn new(machine_id: MachineId) -> (Self, Sender<Delivery>) {
        let (sender, receiver) = mpsc::channel(16);
        let tap = Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
            session: Arc::new(TapSession::new(machine_id)),
        };
        (tap, sender)
    }

    pub fn attach(self: Arc<Self>, network_id: NetworkHandle, sender: Sender<Delivery>) {
        self.session.clone().attach(network_id, sender);
    }
}

impl Protocol for Tap {
    fn id(self: Arc<Self>) -> ProtocolId {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<SharedSession, Box<dyn Error>> {
        Ok(self.session.clone())
    }

    fn listen(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn demux(
        self: Arc<Self>,
        _message: Message,
        _caller: SharedSession,
        _context: Context,
    ) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no
        // protocols under this one that would ask Tap to demux a message and
        // because, semantically, demux chooses one of its own sessions to
        // respond to the message. We want Tap to immediatly forward incoming
        // messages to a higher-up protocol.
        panic!("Cannot demux on a Tap")
    }

    fn start(
        self: Arc<Self>,
        context: Context,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        self.session.clone().start(context.clone())?;
        let mut receiver = self.receiver.lock().unwrap().take().unwrap();
        tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                match self
                    .session
                    .clone()
                    .receive_delivery(delivery, context.clone())
                {
                    Ok(()) => {}
                    Err(e) => println!("{}", e),
                }
            }
        });
        Ok(())
    }
}
