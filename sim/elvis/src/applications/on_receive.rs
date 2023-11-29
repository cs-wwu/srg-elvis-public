use std::sync::{Arc, Mutex};

use elvis_core::{
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, Tcp, Udp},
    Control, Machine, Message, Protocol, Session, Shutdown, Transport,
};
use tokio::sync::Barrier;

/// An application that runs a Fn every time it receives a message.
///
/// # Example
/// ```
/// # use elvis_core::{protocols::*, machine::*};
/// # use elvis::applications::OnReceive;
/// # use elvis_core::IpTable;
/// let fn_to_run = |message, _context| println!("received message: {message}");
/// let _puter = new_machine![
/// OnReceive::new(fn_to_run, Endpoint::new(0.into(), 0xfefe)),
/// Udp::new(),
/// Ipv4::new(IpTable::new()),
/// Pci::new([])
/// ];
/// ```
pub struct OnReceive {
    /// The function to run when a message is received.
    fn_to_run: Mutex<Box<dyn FnMut(Message, Control) + Send>>,
    /// The address we listen for a message on
    local_endpoint: Endpoint,
    transport: Transport,
}

impl OnReceive {
    /// Creates a new OnReceive instance.
    ///
    /// `fn_to_run` will be run whenever a message is received.
    /// The received message and context will be passed to the Fn.
    pub fn new<F>(fn_to_run: F, local_endpoint: Endpoint) -> Self
    where
        F: FnMut(Message, Control) + Send + 'static,
    {
        OnReceive {
            fn_to_run: Mutex::new(Box::new(fn_to_run)),
            local_endpoint,
            transport: Transport::Udp,
        }
    }

    /// Set the transport protocol to use
    pub fn transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }
}

#[async_trait::async_trait]
impl Protocol for OnReceive {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        match self.transport {
            Transport::Tcp => {
                machine
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(self.id(), self.local_endpoint, machine)
                    .unwrap();
            }
            Transport::Udp => {
                machine
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(self.id(), self.local_endpoint, machine)
                    .unwrap();
            }
        }
        initialize.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        context: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        // Run the function of this OnReceive
        let result = self.fn_to_run.lock();
        match result {
            Ok(mut fn_to_run) => {
                fn_to_run(message, context);
                Ok(())
            }
            Err(_) => {
                tracing::error!("Attempted to run function which panicked earlier");
                Err(DemuxError::Other)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use elvis_core::{machine::*, protocols::*, IpTable};

    use crate::applications::OnReceive;

    #[tokio::test(flavor = "multi_thread")]
    async fn doctest() {
        let fn_to_run = |message, _context| println!("received message: {message}");
        let _puter = new_machine![
            OnReceive::new(fn_to_run, Endpoint::new(0.into(), 0xfefe)),
            Udp::new(),
            Ipv4::new(IpTable::new()),
            Pci::new([])
        ];
    }
}
