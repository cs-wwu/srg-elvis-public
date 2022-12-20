//! The base-level protocol that communicates directly with networks.

use crate::{
    control::{Key, Primitive},
    internet::NetworkHandle,
    machine::MachineId,
    message::Message,
    network::Delivery,
    protocol::{Context, DemuxError, ListenError, OpenError, ProtocolId, QueryError, StartError},
    session::SharedSession,
    Control, Protocol,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Barrier,
};

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
    /// The channel on which to receive messages sent to this machine.
    receiver: Arc<Mutex<Option<Receiver<Delivery>>>>,
    /// The session used for sending messages from this machine.
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

    /// Attach this machine to the given network.
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
    ) -> Result<SharedSession, OpenError> {
        Ok(self.session.clone())
    }

    fn listen(
        self: Arc<Self>,
        _upstream: ProtocolId,
        _participants: Control,
        _context: Context,
    ) -> Result<(), ListenError> {
        Ok(())
    }

    fn demux(
        self: Arc<Self>,
        _message: Message,
        _caller: SharedSession,
        _context: Context,
    ) -> Result<(), DemuxError> {
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
        initialized: Arc<Barrier>,
    ) -> Result<(), StartError> {
        // Move the channel into the task. It cannot not be accessed from
        // `self` after this point.
        let mut receiver = self.receiver.lock().unwrap().take().unwrap();
        tokio::spawn(async move {
            initialized.wait().await;
            // Repeatedly receive messages and pass them up the stack
            while let Some(delivery) = receiver.recv().await {
                // Ignore failed deliveries. Rely on sessions to report errors
                // via tracing.
                let _ = self
                    .session
                    .clone()
                    .receive_delivery(delivery, context.clone());
            }
        });
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        // TODO(hardint): Add support for querying the MTU
        match key {
            MACHINE_ID_KEY => Ok(self.session.machine_id.into()),
            _ => Err(QueryError::NonexistentKey),
        }
    }
}
