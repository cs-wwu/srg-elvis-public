use super::{Pci, PciMonitors};
use crate::{
    control::{Key, Primitive},
    machine::{PciSlot, ProtocolMap},
    message::Message,
    network::{Delivery, Tap},
    protocol::{Context, DemuxError},
    session::{QueryError, SendError},
    Id, Network, Session, Shutdown,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{broadcast::error::RecvError, Barrier};

/// The session type for a [`Tap`](super::Tap).
pub struct PciSession {
    tap: Tap,
    index: PciSlot,
    monitors: PciMonitors,
}

impl PciSession {
    /// Creates a new Tap session
    pub(super) fn new(tap: Tap, index: u32, monitors: PciMonitors) -> Self {
        Self {
            tap,
            index,
            monitors,
        }
    }

    /// Called by the owning [`Pci`] protocol at the beginning of the simulation
    /// to start the contained tap running
    pub(super) fn start(
        self: Arc<Self>,
        protocols: ProtocolMap,
        barrier: Arc<Barrier>,
        shutdown: Shutdown,
    ) {
        let mut direct_receiver = self.tap.unicast_receiver.write().unwrap().take().unwrap();
        let mut broadcast_receiver = self.tap.broadcast.write().unwrap().take().unwrap();
        let context = Context::new(protocols);
        let me = self.clone();
        let monitors = self.monitors.clone();
        tokio::spawn(monitors.receive.instrument(async move {
            barrier.wait().await;
            let mut shutdown_receiver = shutdown.receiver();
            loop {
                let context = context.clone();
                tokio::select! {
                    message = monitors.channel_recv.instrument(direct_receiver.recv()) => {
                        me.clone().receive_direct(message, context).await;
                    }
                    message = broadcast_receiver.recv() => {
                        me.clone().receive_broadcast(message, context).await;
                    }
                    _ = shutdown_receiver.recv() => break,
                }
            }
        }));
    }

    async fn receive_direct(self: Arc<Self>, delivery: Option<Delivery>, context: Context) {
        if let Some(delivery) = delivery {
            match self.receive_delivery(delivery, context).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to receive on direct network: {}", e);
                }
            }
        }
    }

    async fn receive_broadcast(
        self: Arc<Self>,
        delivery: Result<Delivery, RecvError>,
        context: Context,
    ) {
        match delivery {
            Ok(delivery) => match self.receive_delivery(delivery, context).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to receive on a broadcast network: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Broadcast receive error: {}", e);
            }
        }
    }

    /// Called by the owned [`Tap`] to pass a frame from the network up the
    /// protocol stack. We use this instead of [`Session::receive`] because the
    /// tap holds a reference to this session as a concrete type and having
    /// specialized arguments to pass a full network frame to this session is
    /// useful.
    pub(crate) async fn receive_delivery(
        self: Arc<Self>,
        delivery: Delivery,
        mut context: Context,
    ) -> Result<(), ReceiveError> {
        Pci::set_pci_slot(self.index, &mut context.control);
        Network::set_sender(delivery.sender, &mut context.control);
        let protocol = match context.protocol(delivery.protocol) {
            Some(protocol) => protocol,
            None => {
                tracing::error!(
                    "Could not find a protocol for the protocol ID {}",
                    delivery.protocol
                );
                Err(ReceiveError::Protocol(delivery.protocol))?
            }
        };
        protocol.demux(delivery.message, self, context).await?;
        Ok(())
    }
}

#[async_trait]
impl Session for PciSession {
    #[tracing::instrument(name = "PciSession::send", skip_all)]
    async fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), SendError> {
        let protocol = match Network::get_protocol(&context.control) {
            Ok(protocol) => protocol,
            Err(_) => {
                tracing::error!("Protocol missing from context");
                Err(SendError::MissingContext)?
            }
        };
        let destination = Network::get_destination(&context.control).ok();

        if message.len() > self.tap.mtu as usize {
            tracing::error!("Attempted to send a message larger than the network can handle");
            Err(SendError::Mtu(self.tap.mtu))?
        }

        let funnel = self.tap.delivery_sender.clone();
        let delivery = Delivery {
            message,
            sender: self.tap.mac,
            destination,
            protocol,
        };

        match funnel.send(delivery).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to send on direct network: {}", e);
            }
        }

        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Pci::MTU_QUERY_KEY => Ok(self.tap.mtu.into()),
            _ => Err(QueryError::MissingKey),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    #[error("Could not find a protocol for the given id: {0}")]
    Protocol(Id),
    #[error("{0}")]
    Demux(#[from] DemuxError),
}
