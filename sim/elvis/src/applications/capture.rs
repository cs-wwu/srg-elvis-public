//! An application that shuts down the simulation once it receives some messages.

use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Tcp, Udp},
    shutdown::ExitStatus,
    Control, Protocol, Session, Shutdown, Transport,
};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Barrier;

/// An application that can be configured to shut down the simulation when:
///
/// * It receives a certain number of messages OR
/// * It receives a certain set of messages
///
/// The application also stores messages it receives
/// (these can be accessed using [`Capture::received`]).
///
/// If you want to use multiple `Capture`s in a single sim,
/// and don't want the simulation to shut down until all of them
/// have finished receiving, see [`CapFactory`].
#[derive(Debug)]
pub struct Capture {
    /// The messages we expect to receive
    expected: MCExpected,
    /// The messages that were received, if any
    received: Mutex<MCReceived>,

    /// The channel we send on to shut down the simulation
    shutdown: OnceLock<Shutdown>,
    exit_status: Option<u32>,

    endpoint: Endpoint,
    /// The transport protocol to use
    transport: Transport,

    /// The number of captures connected to this one
    /// (including itself) which have not finished capturing yet.
    remaining: Arc<Mutex<u32>>,
}

/// Helper enum, see [`Capture::expected`]
#[derive(Debug)]
enum MCExpected {
    /// Indicates this capture expects to receive a certain number of messages
    Count(u32),
    /// Indicates this capture expects to receive a certain set of messages
    Set(Vec<Message>),
}

/// Helper struct, see [`Capture::received`]
#[derive(Debug)]
struct MCReceived {
    /// Will be true if this capture has finished receiving
    pub done: bool,
    /// The messages received
    pub messages: Vec<Message>,
}

/// A factory used to create multiple connected [`Capture`]s.
/// If multiple captures are created from the same factory,
/// they will only shut down the simulation once *all of them*
/// have finished receiving their messages.
///
/// If you only need to make 1 capture that isn't connected,
/// use [`Capture::new_1`] and other functions.
#[derive(Debug)]
pub struct CapFactory {
    /// A shared number used to keep track of how many captures have yet to
    /// receive their messages.
    count: Arc<Mutex<u32>>,
}

impl CapFactory {
    /// Creates a new `CapFactory`.
    pub fn new() -> CapFactory {
        Self {
            count: Arc::new(Mutex::new(0)),
        }
    }

    /// Creates a new capture that will finish when it receives the given number of messages.
    pub fn build(&self, endpoint: Endpoint, n: u32) -> Capture {
        // Increase number of remaining captures
        *self.count.lock().unwrap() += 1;

        let received = MCReceived {
            done: false,
            messages: Vec::with_capacity(n as usize),
        };

        Capture {
            received: Mutex::new(received),
            shutdown: OnceLock::new(),
            endpoint,
            expected: MCExpected::Count(n),
            transport: Transport::Udp,
            exit_status: None,
            remaining: Arc::clone(&self.count),
        }
    }

    /// Creates a new capture that will finish when it receives the given number of messages.
    pub fn build_set(&self, endpoint: Endpoint, messages: Vec<Message>) -> Capture {
        let mut result = self.build(endpoint, 0);
        result.expected = MCExpected::Set(messages);
        result
    }
}

impl Capture {
    /// Creates a new capture that will finish when it receives the given number of messages.
    pub fn new(endpoint: Endpoint, n: u32) -> Self {
        CapFactory::new().build(endpoint, n)
    }

    /// Creates a new capture that will finish when it has received all the messages
    /// in the given `messages` set.
    pub fn new_set(endpoint: Endpoint, messages: Vec<Message>) -> Self {
        CapFactory::new().build_set(endpoint, messages)
    }

    /// Returns the first message that was received.
    pub fn first_msg(&self) -> Option<Message> {
        self.received().next()
    }

    /// Returns an iterator over all the messages that were received.
    pub fn received(&self) -> impl DoubleEndedIterator<Item = Message> {
        self.received
            .lock()
            .expect("mutex should not be poisoned")
            .messages
            .clone()
            .into_iter()
    }

    /// Set the transport protocol (TCP or UDP) that this Capture should use.
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// Set the exit status for capture to return with on shutdown.
    pub fn exit_status(mut self, status: u32) -> Self {
        self.exit_status = Some(status);
        self
    }
}

#[async_trait::async_trait]
impl Protocol for Capture {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let broadcast_endpoint = Endpoint::new(Ipv4Address::SUBNET, self.endpoint.port);

        match self.transport {
            Transport::Tcp => {
                protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols)
                    .unwrap();
            }
            Transport::Udp => {
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, protocols.clone())
                    .unwrap();

                // listen on broadcast
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), broadcast_endpoint, protocols)
                    .unwrap();
            }
        }

        self.shutdown
            .set(shutdown)
            .expect("Capture should only be started once");
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        println!("received message of len: {}", message.len());
        // lock received messages for duration of demux
        let mut guard = self.received.lock().unwrap();

        guard.messages.push(message);

        // exit if we're done receiving
        if guard.done {
            return Ok(());
        }

        let finished: bool = match &self.expected {
            MCExpected::Count(n) => guard.messages.len() >= *n as usize,
            // check if the received messages contains all the expected messages
            MCExpected::Set(set) => set.iter().all(|e| guard.messages.contains(e)),
        };

        if finished {
            guard.done = true;
            let exit_status = match self.exit_status {
                Some(num) => ExitStatus::Status(num),
                None => ExitStatus::Exited,
            };
            // decrement the counter of captures remaining
            let mut remaining_guard = self
                .remaining
                .lock()
                .expect("mutex should not be poisioned");
            *remaining_guard -= 1;
            if *remaining_guard == 0 {
                self.shutdown
                    .get()
                    .expect("simulation should be started before demux!")
                    .shut_down_with_status(exit_status);
            }
            drop(remaining_guard);
        }

        drop(guard);
        Ok(())
    }
}
