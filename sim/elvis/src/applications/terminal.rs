//! The terminal protocol is an application that lives above UDP.
//! The application communicates with an actual port on the real-world machine running ELVIS
//! and sends and receives messages over this port via TCP communication with a command terminal.

use elvis_core::*;
use elvis_core::machine::*;
use elvis_core::session::Session;
use elvis_core::protocol::*;
use tokio::sync::Barrier;
use std::sync::{Arc, RwLock};
use std::any::*;



struct Terminal {
    /// The queue of messages received (qpush) by the application that can be
    /// returned (qpop) when a fetch request is made.
    msg_queue: RwLock<Vec<String>>,
}

impl Terminal {
    fn run(
        // The real-world port to communicate over
        port: String,
        protocols: ProtocolMap,
    ) {

    }

    /// Returns and removes the first element in the msg_queue
    fn qpop() -> Option<String> {
        let mut q: Vec<String> = msg_queue
            .write()
            .unwrap();

        // Need to test to see what happens with empty queue
        popped = q.remove(0);

        match popped {
            Some(x) => popped,
            None    => println!("No messages in queue!"),
        }

    }

    /// Adds an element to the end of the msg_queue
    fn qpush(
        msg: String
    ) {
        
    }
}

#[async_trait::async_trait]
impl Protocol for Terminal {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Initialize message queue
        msg_queue = VecDeque::new();

        // tokio spawn

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}