use std::{
    any::TypeId,
    sync::{Arc, Mutex},
};

use elvis_core::{
    machine::ProtocolMap,
    protocols::{
        user_process::{Application, ApplicationError},
        Endpoint, Tcp, Udp, UserProcess,
    },
    Control, Message, Shutdown, Transport,
};
use tokio::sync::Barrier;

/// An application that runs a Fn every time it receives a message.
///
/// # Example
/// ```
/// # use elvis_core::{protocols::*, machine::*};
/// # use elvis::applications::OnReceive;
/// let fn_to_run = |message, _context| println!("received message: {message}");
/// let _puter = Machine::new(
///     ProtocolMapBuilder::new()
/// .with(OnReceive::new(fn_to_run, Endpoint::new(0.into(), 0xfefe)))
/// .with(Udp::new())
/// .with(Ipv4::new(std::iter::empty().collect()))
/// .with(Pci::new([]))
/// .build());
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
    pub fn new<F>(fn_to_run: F, local_endpoint: Endpoint) -> UserProcess<Self>
    where
        F: FnMut(Message, Control) + Send + 'static,
    {
        UserProcess::new(OnReceive {
            fn_to_run: Mutex::new(Box::new(fn_to_run)),
            local_endpoint,
            transport: Transport::Udp,
        })
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
    fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        match self.transport {
            Transport::Tcp => {
                protocols
                    .protocol::<Tcp>()
                    .unwrap()
                    .listen(
                        TypeId::of::<UserProcess<Self>>(),
                        self.local_endpoint,
                        protocols,
                    )
                    .unwrap();
            }
            Transport::Udp => {
                protocols
                    .protocol::<Udp>()
                    .unwrap()
                    .listen(
                        TypeId::of::<UserProcess<Self>>(),
                        self.local_endpoint,
                        protocols,
                    )
                    .unwrap();
            }
        }
        tokio::spawn(async move {
            initialize.wait().await;
        });
        Ok(())
    }

    fn receive(
        &self,
        message: Message,
        context: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
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
    use elvis_core::{machine::*, protocols::*};

    use crate::applications::OnReceive;

    #[tokio::test]
    async fn doctest() {
        let fn_to_run = |message, _context| println!("received message: {message}");
        let _puter = new_machine![
            OnReceive::new(fn_to_run, Endpoint::new(0.into(), 0xfefe)),
            Udp::new(),
            Ipv4::new(std::iter::empty().collect()),
            Pci::new([])
        ];
    }
}
