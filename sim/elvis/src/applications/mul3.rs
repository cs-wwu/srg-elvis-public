use std::sync::Arc;

use elvis_core::{
    machine::ProtocolMap,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::Ipv4Address, Endpoint, Endpoints, Udp},
    Control, Message, Protocol, Session, Shutdown,
};

/// The port associated with the Mul3 server.
pub const MUL3PORT: u16 = 25565;

/// An Application which sends out a number.
/// When it receives that number multiplied by 3,
/// it shuts down the simulation.
pub struct Mul3Client {
    /// The IP address of this client.
    local_end: Endpoint,
    /// The IP address of the server to send the number to.
    server_ip: Ipv4Address,
    /// The number we'll send out when the simulation starts.
    number: u8,
    /// The Shutdown object, so we can shut down the simulation
    /// once we receive the number times 3.
    /// The OnceLock is used so that the Shutdown can be inserted into
    /// this struct while `.start` is being run.
    /// It's like a RwLock, but it can only be written to once.
    shutdown: std::sync::OnceLock<Shutdown>,
}

impl Mul3Client {
    /// Creates a new Mul3Client.
    ///
    /// # Parameters
    ///
    /// * `local_end` - the IP address, port pair for this machine.
    /// * `server_ip` - the IP address of the Mul3Server.
    /// * `number` - the number to send to the Mul3Server, and to expect to be multiplied by 3.
    pub fn new(local_end: Endpoint, server_ip: Ipv4Address, number: u8) -> Mul3Client {
        Mul3Client {
            local_end,
            server_ip,
            number,
            shutdown: std::sync::OnceLock::new(),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for Mul3Client {
    async fn start(
        &self,
        // This is an object that can be used to shut down the simulation.
        shutdown: Shutdown,
        // The `Barrier` is a type of "synchronization primitive".
        // It is used to make sure that several threads can wait for something to happen
        // before starting.
        // When you call `initialized.wait().await;`,
        // it waits for EVERY protocol in the simulation to run
        // `initialized.wait().await` before continuing.
        initialized: Arc<tokio::sync::Barrier>,
        // A map of all the protocols on this machine.
        // You can access other protocols in the machine using `protocols.protocol`.
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Save the Shutdown in the struct, so it can be used later
        // to shut down the simulation.
        let _ = self.shutdown.set(shutdown);

        // Wait for all other protocols to do `initialized.wait().await`
        // before moving on.
        // When writing a protocol, you must ALWAYS do initialized.wait().await,
        // or it may stop other machines from being initialized!
        initialized.wait().await;

        // Create a (remote IP address, remote port) pair to represent the server.
        let remote = Endpoint::new(self.server_ip, MUL3PORT);

        // Create a pair of endpoints for use in udp.open_and_listen.
        let endpoints: Endpoints = Endpoints {
            local: self.local_end,
            remote,
        };

        // Get the Udp on this machine.
        let udp = protocols
            .protocol::<Udp>()
            .expect("This machine should have Udp");

        // Create a Udp session so we can send our number to the server.
        let session = udp
            .open_and_listen(
                // Our protocol ID, so Udp knows to send the messages to us.
                self.id(),
                // The endpoints, to specify where we are sending our message to and from.
                endpoints,
                protocols.clone(),
            )
            .await
            .expect("Session should not fail to open");

        // Send the number to the server.
        let message = Message::from([self.number]);
        let _ = session.send(message, protocols);

        Ok(())
    }

    // This method is called by Udp when we receive a message
    // that we listened for: in this case,
    // it is called when we receive a message for self.local_end.
    fn demux(
        &self,
        // The message we received, after the UDP (and IP) headers are removed.
        message: Message,
        // The downstream (protocol below us) session.
        // Sessions are a little bit like Unix sockets.
        // They represent a connection between us and another computer.
        // They have a Session::send that you can use to send a message back.
        // (We don't use it here.)
        _caller: Arc<dyn Session>,
        // Additional details associated with the message,
        // such as the IP address of the sender.
        _control: Control,
        // The other protocols on this machine.
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Gets the message's first byte.
        let message_first_byte = match message.iter().next() {
            Some(n) => n,
            None => {
                tracing::error!("Message was empty");
                return Ok(());
            }
        };

        if message_first_byte == self.number * 3 {
            // Prints a message saying we received the number we expected.
            tracing::info!("received number {}", message_first_byte);

            // Shuts down the simulation.
            self.shutdown
                .get()
                .expect("Simulation should be started so that Mul3Client can shutdown")
                .shut_down();
        } else {
            // Logs an error message.
            tracing::error!(
                "Client received {} which is not {} * 3",
                message_first_byte,
                self.number
            );
        }
        Ok(())
    }
}
