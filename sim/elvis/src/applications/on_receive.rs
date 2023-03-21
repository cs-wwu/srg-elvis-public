use std::sync::{Arc, Mutex};

use elvis_core::{
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        user_process::{Application, ApplicationError},
        Ipv4, Tcp, Udp, UserProcess,
    },
    Control, Id, Message, ProtocolMap,
};
use tokio::sync::{mpsc::Sender, Barrier};

use super::Transport;

/// An application that runs a Fn every time it receives a message.
///
/// # Example
/// ```
/// # use elvis_core::{Machine, protocols::{ipv4::{Ipv4Address, Ipv4}, Udp, Pci}, protocol::SharedProtocol};
/// # use elvis::applications::OnReceive;
/// let fn_to_run = |message, _context| println!("received message: {message}");
///
/// let _puter = Machine::new([
///    OnReceive::new(fn_to_run, Ipv4Address::from(0), 0xfefe).shared() as SharedProtocol,
///    Udp::new().shared(),
///    Ipv4::new(Default::default()).shared(),
///    Pci::new([]).shared(),
/// ]);
/// ```
pub struct OnReceive {
    /// The function to run when a message is received.
    fn_to_run: Mutex<Box<dyn FnMut(Message, Context) + Send>>,
    /// The address we listen for a message on
    ip_address: Ipv4Address,
    /// The port we listen for a message on
    port: u16,
    /// The transport protocol to use
    transport: Transport,
}

impl OnReceive {
    /// Creates a new OnReceive instance.
    ///
    /// `fn_to_run` will be run whenever a message is received.
    /// The received message and context will be passed to the Fn.
    pub fn new<F>(fn_to_run: F, ip_address: Ipv4Address, port: u16) -> OnReceive
    where
        F: FnMut(Message, Context) + Send + 'static,
    {
        OnReceive {
            fn_to_run: Mutex::new(Box::new(fn_to_run)),
            ip_address,
            port,
            transport: Transport::Udp,
        }
    }

    /// Creates a new OnReceive behind a shared handle.
    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }

    /// Set the transport protocol to use
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

impl Application for OnReceive {
    const ID: Id = Id::from_string("OnReceive");

    fn start(
        &self,
        _shutdown: Sender<()>,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // This code was copy and pasted from [`Capture`](elvis::applications::Capture). Haha.
        let mut participants = Control::new();
        Ipv4::set_local_address(self.ip_address, &mut participants);
        match self.transport {
            Transport::Udp => Udp::set_local_port(self.port, &mut participants),
            Transport::Tcp => Tcp::set_local_port(self.port, &mut participants),
        }
        protocols
            .protocol(self.transport.id())
            .expect("No such protocol")
            .listen(Self::ID, participants, protocols)?;
        tokio::spawn(async move {
            initialize.wait().await;
        });
        Ok(())
    }

    fn receive(&self, message: Message, context: Context) -> Result<(), ApplicationError> {
        // Run the function of this OnReceive
        let result = self.fn_to_run.lock();
        match result {
            Ok(mut fn_to_run) => {
                fn_to_run(message, context);
                Ok(())
            }
            Err(_) => {
                tracing::error!("Attempted to run function which panicked earlier");
                Err(ApplicationError::Other)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use elvis_core::{
        protocol::SharedProtocol,
        protocols::{
            ipv4::{Ipv4, Ipv4Address},
            Pci, Udp,
        },
        Machine,
    };

    use crate::applications::OnReceive;

    #[tokio::test]
    async fn doctest() {
        let fn_to_run = |message, _context| println!("received message: {message}");
        let _puter = Machine::new([
            OnReceive::new(fn_to_run, Ipv4Address::from(0), 0xfefe).shared() as SharedProtocol,
            Udp::new().shared(),
            Ipv4::new(Default::default()).shared(),
            Pci::new([]).shared(),
        ]);
    }
}
