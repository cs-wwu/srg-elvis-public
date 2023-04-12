//! A special Protocol that can be used to listen to messages received by another protocol.

use crate::{
    control::{Key, Primitive},
    protocol::{
        Context, DemuxError, ListenError, OpenError, QueryError, SharedProtocol, StartError,
    },
    session::{self, SharedSession},
    Control, Id, Message, Protocol, ProtocolMap, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, Barrier};

type Sender = mpsc::UnboundedSender<(Message, Context)>;
type Receiver = mpsc::UnboundedReceiver<(Message, Context)>;

/// "SubWrap" is short for "Subscribeable wrapper." A special Protocol that can be used to listen to messages received by another protocol.
///
/// A protocol can be wrapped in a SubWrap using [`SubWrap::new`].
///
/// Whenever [`id`], [`demux`], [`open`], [`listen`], or [`query`] is called, these functions are also called on the inner protocol.
///
/// `SubWrap` has 2 special functions: [`subscribe_demux`] and [`subscribe_send`]. These return receivers that will be sent on
/// whenever `SubWrap::demux` and [`SubWrapSession::send`] are called, respectively.
///
/// # Example
///
/// ```
/// # use elvis_core::{
/// #    protocol::SharedProtocol,
/// #    protocols::{Ipv4, Pci, SubWrap},
/// #    run_internet, Machine, Network,
/// # };
///
/// #[tokio::main]
/// async fn main() {
///     let network = Network::basic();
///
///     let mut wrapper = SubWrap::new(Pci::new([network.clone()]));
///
///     let mut message_recv = wrapper.subscribe_demux();
///
///     let machine = Machine::new([
///         Ipv4::new([].into_iter().collect()).shared() as SharedProtocol,
///         wrapper.shared(),
///     ]);
///
///     tokio::spawn(run_internet(vec![machine], vec![network]));
///
///     // Receive messages from demux:
///     tokio::spawn(async move { message_recv.recv().await });
/// }
/// ```
/// [`id`]: SubWrap::id
/// [`demux`]: SubWrap::demux
/// [`open`]: SubWrap::open
/// [`listen`]: SubWrap::listen
/// [`query`]: SubWrap::query
/// [`subscribe_demux`]: SubWrap::subscribe_demux
/// [`subscribe_send`]: SubWrap::subscribe_send
pub struct SubWrap {
    /// The protocol wrapped by this one
    inner: SharedProtocol,
    /// send on these when the demux method is called
    demux_senders: Vec<Sender>,
    /// send on these when the send method is called on one of this wrapper's sessions
    send_senders: Arc<RwLock<Vec<Sender>>>,
}

impl SubWrap {
    /// Creates a new SubWrap from `inner`.
    pub fn new(inner: impl Protocol + Sync + Send + 'static) -> Self {
        SubWrap {
            inner: Arc::new(inner),
            demux_senders: Default::default(),
            send_senders: Default::default(),
        }
    }

    /// Creates a new shared handle to an instance of the protocol.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Returns an UnboundedReceiver.
    /// Whenever `demux` is called on this wrapper, the message will be sent to this receiver.
    /// A copy of the message's context will be included.
    ///
    /// # Memory leaks
    ///
    /// This UnboundedReceiver will store an unlimited number of messages.
    /// To prevent them from taking up all the memory, you must do 1 of 3 things:
    /// * [`recv()`](mpsc::UnboundedReceiver::recv) them, or
    /// * [`close()`](mpsc::UnboundedReceiver::close) the receiver, or
    /// * drop the Receiver.
    pub fn subscribe_demux(&mut self) -> Receiver {
        let (send, recv) = mpsc::unbounded_channel();
        self.demux_senders.push(send);
        recv
    }

    /// Returns an UnboundedReceiver.
    /// Whenever `send` is called on one of this wrapper's sessions, the message will be sent to this receiver.
    /// A copy of the message's context will be included.
    ///
    /// NOTE: only sessions created with [`SubWrap::open`] will send to this receiver.
    /// If the inner protocol calls send on its own sessions, they will not be tracked.
    ///
    /// # Memory leaks
    ///
    /// This UnboundedReceiver will store an unlimited number of messages.
    /// To prevent them from taking up all the memory, you must do 1 of 3 things:
    /// * [`recv()`](mpsc::UnboundedReceiver::recv) them, or
    /// * [`close()`](mpsc::UnboundedReceiver::close) the receiver, or
    /// * drop the Receiver.
    pub fn subscribe_send(&mut self) -> Receiver {
        let (send, recv) = mpsc::unbounded_channel();
        self.send_senders.write().unwrap().push(send);
        recv
    }
}

impl Protocol for SubWrap {
    fn id(&self) -> Id {
        self.inner.clone().id()
    }

    /// Calls [`start`](Protocol::start) on the inner protocol.
    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        self.inner.clone().start(shutdown, initialized, protocols)
    }

    /// Calls [`open`](Protocol::open) on the inner protocol.
    /// Wraps the resulting session with a [`SubWrapSession`].
    fn open(
        &self,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        let sesh = self.inner.clone().open(upstream, participants, protocols)?;
        let result = SubWrapSession {
            send_senders: self.send_senders.clone(),
            inner: sesh,
        };
        Ok(Arc::new(result))
    }

    /// Calls [`listen`](Protocol::listen) on the inner protocol.
    fn listen(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        self.inner.clone().listen(upstream, participants, protocols)
    }

    /// Sends the message to all receivers obtained with [`subscribe_demux`](Self::subscribe_demux), then calls `demux` on the inner protocol.
    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        send_on_all(&self.demux_senders, &message, &context);
        self.inner.clone().demux(message, caller, context)
    }

    /// Calls [`query`](Protocol::query) on the inner protocol and returns the result.
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.inner.clone().query(key)
    }
}

/// Session for [`SubWrap`].
pub struct SubWrapSession {
    /// The SubWrap protocol that created this session
    send_senders: Arc<RwLock<Vec<Sender>>>,
    inner: SharedSession,
}

impl Session for SubWrapSession {
    /// Sends the message to all receivers created by [`subscribe_send`](SubWrap::subscribe_send),
    /// then calls [`send`](Session::send) on the inner session.
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), session::SendError> {
        let send_senders = self.send_senders.read().unwrap();
        send_on_all(send_senders.as_slice(), &message, &context);
        self.inner.clone().send(message, context)
    }

    /// Calls [`query`](Session::query) on the inner Session, then returns the result.
    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, session::QueryError> {
        self.inner.clone().query(key)
    }
}

fn send_on_all(senders: &[Sender], message: &Message, context: &Context) {
    for sender in senders {
        if !sender.is_closed() {
            let _ = sender.send((message.clone(), context.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        protocol::SharedProtocol,
        protocols::{Ipv4, Pci, SubWrap},
        run_internet, Machine, Network,
    };

    #[tokio::test]
    async fn doctest_1() {
        let network = Network::basic();

        let mut wrapper = SubWrap::new(Pci::new([network.clone()]));

        let mut message_recv = wrapper.subscribe_demux();

        let machine = Machine::new([
            Ipv4::new([].into_iter().collect()).shared() as SharedProtocol,
            wrapper.shared(),
        ]);

        tokio::spawn(run_internet(vec![machine], vec![network]));

        // Receive messages from demux:
        tokio::spawn(async move { message_recv.recv().await });
    }
}
