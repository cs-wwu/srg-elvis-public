use super::{Delivery, Mac, Mtu};
use crate::{
    machine::ProtocolMap, protocol::Context, protocols::pci::PciSession, session::SendError, Id,
    Message, Network,
};
use std::sync::{Arc, RwLock};
use tokio::{
    sync::{broadcast::error::RecvError, mpsc, Barrier},
    time::sleep,
};

/// An access point to a [`Network`]. A tap can be created by calling
/// [`Network::tap`]. Taps should be added to a [`crate::protocols::Pci`]
/// protocol to allow a [`Machine`](crate::Machine) to access the network.
pub struct Tap {
    network: Arc<Network>,
    mac: Mac,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Delivery>>>>,
}

impl Tap {
    /// Creates a new tap
    pub(super) fn new(network: Arc<Network>, mac: Mac, receiver: mpsc::Receiver<Delivery>) -> Self {
        Self {
            network,
            mac,
            direct_receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }

    /// Called at the beginning of the simulation to start the tap running.
    pub(crate) fn start(&self, environment: TapEnvironment, barrier: Arc<Barrier>) {
        let mut direct_receiver = self.direct_receiver.write().unwrap().take().unwrap();
        let mut broadcast_receiver = self.network.broadcast.subscribe();
        tokio::spawn(async move {
            barrier.wait().await;
            loop {
                tokio::select! {
                    message = direct_receiver.recv() => {
                        receive_direct(message, environment.clone());
                    }
                    message = broadcast_receiver.recv() => {
                        receive_broadcast(message, environment.clone());
                    }
                }
            }
        });
    }

    /// Send a message over the network
    pub(crate) fn send(
        &self,
        message: Message,
        destination: Option<Mac>,
        protocol: Id,
    ) -> Result<(), SendError> {
        if let Some(mtu) = self.network.mtu {
            if message.len() > mtu as usize {
                tracing::error!("Attempted to send a message larger than the network can handle");
                Err(SendError::Mtu(mtu))?
            }
        }

        let latency = self.network.latency;
        let funnel = self.network.delivery_sender.clone();
        let delivery = Delivery {
            message,
            sender: self.mac,
            destination,
            protocol,
        };

        tokio::spawn(async move {
            if let Some(latency) = latency {
                sleep(latency).await;
            }
            match funnel.send(delivery).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to send on direct network: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Gets the maximum transmission unit of the attached network
    pub(crate) fn mtu(&self) -> Option<Mtu> {
        self.network.mtu
    }
}

fn receive_direct(delivery: Option<Delivery>, environment: TapEnvironment) {
    if let Some(delivery) = delivery {
        let context = environment.context();
        match environment.session.receive_pci(delivery, context) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to receive on direct network: {}", e);
            }
        }
    }
}

fn receive_broadcast(delivery: Result<Delivery, RecvError>, environment: TapEnvironment) {
    match delivery {
        Ok(delivery) => {
            let context = environment.context();
            match environment.session.receive_pci(delivery, context) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to receive on a broadcast network: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Broadcast receive error: {}", e);
        }
    }
}

/// Allows a [`Tap`] requires to get information about its environment
#[derive(Clone)]
pub(crate) struct TapEnvironment {
    pub protocols: ProtocolMap,
    pub session: Arc<PciSession>,
}

impl TapEnvironment {
    pub fn new(protocols: ProtocolMap, session: Arc<PciSession>) -> Self {
        Self { protocols, session }
    }

    pub fn context(&self) -> Context {
        Context::new(self.protocols.clone())
    }
}
