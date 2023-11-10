//! An application that shuts down the simulation once it receives some messages.

use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Tcp, Udp},
    shutdown::ExitStatus,
    Control, Machine, Protocol, Session, Shutdown, Transport,
};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Barrier;

/// An application that can be configured to shut down the simulation when:
///
/// * It receives a certain number of messages OR
/// * It receives some specific message
///
/// The application also stores messages it receives
/// (these can be accessed using [`Capture::message`]).
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
    /// Indicates this capture expects to receive a certain message
    Msg(Message),
}

/// Helper struct, see [`Capture::received`]
#[derive(Debug)]
struct MCReceived {
    /// Will be true if this capture has finished receiving
    pub done: bool,
    /// The messages received, concatenated
    pub messages: Option<Message>,
    /// The number of times demux was called on us
    pub amount: u64,
}

/// A factory used to create multiple connected [`Capture`]s.
/// If multiple captures are created from the same factory,
/// they will only shut down the simulation once *all of them*
/// have finished receiving their messages.
///
/// If you only need to make 1 capture that isn't connected,
/// use [`Capture::new`] and other functions.
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
            messages: None,
            amount: 0,
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

    /// Creates a new capture that will finish when it receives a bunch of messages that,
    /// when combined together, form the given message.
    pub fn build_msg(&self, endpoint: Endpoint, message: Message) -> Capture {
        let mut result = self.build(endpoint, 0);
        result.expected = MCExpected::Msg(message);
        result
    }
}

impl Default for CapFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture {
    /// Creates a new capture that will finish when it receives the given number of messages.
    pub fn new(endpoint: Endpoint, n: u32) -> Self {
        CapFactory::new().build(endpoint, n)
    }

    /// Creates a new capture that will finish when it has received all the messages
    /// in the given `messages` set.
    pub fn new_msg(endpoint: Endpoint, message: Message) -> Self {
        CapFactory::new().build_msg(endpoint, message)
    }

    /// Returns the messages that were received, all concatenated into 1 message.
    pub fn message(&self) -> Option<Message> {
        self.received.lock().unwrap().messages.clone()
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
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        let broadcast_endpoint = Endpoint::new(Ipv4Address::SUBNET, self.endpoint.port);

        match self.transport {
            Transport::Tcp => {
                machine
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, machine)
                    .unwrap();
            }
            Transport::Udp => {
                machine
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), self.endpoint, machine.clone())
                    .unwrap();

                // listen on broadcast
                machine
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), broadcast_endpoint, machine)
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
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        println!("received message of len: {}", message.len());
        // lock received messages for duration of demux
        let mut guard = self.received.lock().unwrap();
        guard.amount += 1;

        match &mut guard.messages {
            Some(existing) => existing.concatenate(message),
            None => guard.messages = Some(message),
        }

        // exit if we're done receiving
        if guard.done {
            return Ok(());
        }

        let finished: bool = match &self.expected {
            MCExpected::Count(n) => guard.amount >= *n as u64,
            // check if the received messages is equal to the expected message
            MCExpected::Msg(expected) => guard.messages.as_ref() == Some(expected),
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
