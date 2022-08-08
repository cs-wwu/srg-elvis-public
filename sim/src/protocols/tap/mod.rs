//! The base-level protocol that communicates directly with networks.

use crate::core::{
    message::Message, Control, Delivery, Mtu, Postmarked, Protocol, ProtocolContext, ProtocolId,
    SharedSession,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    mem,
    sync::{Arc, Mutex},
};

mod tap_misc;
use futures::{stream::FuturesUnordered, StreamExt};
pub use tap_misc::{NetworkId, NetworkInfo};

mod tap_session;
use tap_session::TapSession;
use tokio::sync::mpsc::{Receiver, Sender};

use self::{tap_misc::TapError, tap_session::SessionId};

/// Represents something akin to an Ethernet tap or a network interface card.
///
/// A tap sits at the bottom of a protocol stack and should be the first
/// responder to messages coming in off the network. It is simply there to
/// specify which protocol should respond to a raw message coming off the
/// network, for example IPv4 or IPv6. The header is very simple, adding only a
/// u32 that specifies the `ProtocolId` of the protocol that should receive the
/// message.
#[derive(Default)]
pub struct Tap {
    receivers: HashMap<crate::core::NetworkId, Receiver<Delivery>>,
    senders: HashMap<crate::core::NetworkId, (Sender<Postmarked>, Mtu)>,
    sessions: HashMap<SessionId, Arc<Mutex<TapSession>>>,
}

impl Tap {
    /// A unique identifier for the protocol.
    pub const ID: ProtocolId = ProtocolId::new(0xdeadbeef);

    /// Creates a new network tap.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn attach(&mut self, network_info: NetworkInfo, network_id: crate::core::NetworkId) {
        let NetworkInfo {
            mtu,
            sender,
            receiver,
        } = network_info;
        match self.receivers.entry(network_id) {
            Entry::Occupied(_) => panic!("Tried attaching the same network twice"),
            Entry::Vacant(entry) => {
                entry.insert(receiver);
            }
        }
        match self.senders.entry(network_id) {
            Entry::Occupied(_) => panic!("Tried attaching the same network twice"),
            Entry::Vacant(entry) => {
                entry.insert((sender, mtu));
            }
        }
    }
}

impl Protocol for Tap {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open(
        &mut self,
        upstream: ProtocolId,
        participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<SharedSession, Box<dyn Error>> {
        let network = NetworkId::get(&participants);
        let session_id = SessionId::new(upstream, network.into());
        match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone().into()),
            Entry::Vacant(entry) => {
                let sender = self.senders.get(&network).unwrap().0.clone();
                let session = Arc::new(Mutex::new(TapSession::new(upstream, sender)));
                entry.insert(session.clone());
                Ok(session.into())
            }
        }
    }

    fn listen(
        &mut self,
        _upstream: ProtocolId,
        _participants: Control,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // This is a no-op because nobody can call open_passive on us anyway
        Ok(())
    }

    fn demux(
        &mut self,
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no
        // protocols under this one that would ask Tap to demux a message and
        // because, semantically, demux chooses one of its own sessions to
        // respond to the message. We want Tap to immediatly forward incoming
        // messages to a higher-up protocol.
        panic!("Cannot demux on a Tap")
    }

    fn start(
        &mut self,
        context: ProtocolContext,
        _shutdown: Sender<()>,
    ) -> Result<(), Box<dyn Error>> {
        // Receivers is not clone, but it is only used here so we take it
        let mut receivers = mem::take(&mut self.receivers);
        let senders = self.senders.clone();
        let mut sessions = self.sessions.clone();
        tokio::spawn(async move {
            // FuturesUnordered allows us to poll incoming messages from all
            // networks
            let mut futures: FuturesUnordered<_> = receivers
                .values_mut()
                .map(|receiver| receiver.recv())
                .collect();
            // Take each incoming message and pass it up
            while let Some(Some(delivery)) = futures.next().await {
                let mut context = context.clone();
                let header = take_header(&delivery.message)
                    .ok_or(TapError::HeaderLength)
                    .unwrap();
                NetworkId::set(&mut context.info, delivery.network);
                let message = delivery.message.slice(8..);
                let session_id = SessionId::new(header, delivery.network.into());
                let session = match sessions.entry(session_id) {
                    Entry::Occupied(entry) => entry.get().clone(),
                    Entry::Vacant(entry) => {
                        let sender = senders.get(&delivery.network).unwrap().0.clone();
                        let session = Arc::new(Mutex::new(TapSession::new(header, sender)));
                        entry.insert(session.clone());
                        session
                    }
                };
                let mut session = SharedSession::from(session);
                match session.receive(message, &mut context) {
                    Ok(()) => {}
                    Err(e) => println!("{}", e),
                }
            }
        });
        Ok(())
    }
}

fn take_header(message: &Message) -> Option<ProtocolId> {
    let mut iter = message.iter();
    Some(
        u64::from_be_bytes([
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
            iter.next()?,
        ])
        .into(),
    )
}
