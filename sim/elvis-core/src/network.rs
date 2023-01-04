//! Provides facilities for [`Machine`](super::Machine)s to communicate.
//!
//! The [`Network`] type is the way for [`Machine`](super::Machine)s to exchange
//! [`Message`]s. When multiple machines are connected to the same network, they
//! can directly send messages to one another by using the
//! [`Pci`](crate::protocols::Pci) protocol. Machines can be added to a network
//! in the following way:
//!
//! - Create the network with the desired properties using the
//!   [`NetworkBuilder`]
//! - Call [`Network::tap`] on the the network to get a [`Tap`]. A tap is a an
//!   access point to the network that can be used to send and receive messages.
//!   Each tap also acts as an identifier so that peers on the network can
//!   exchange messages directly.
//! - Add a [`Pci`](crate::protocols::Pci) protocol to the machine that wants to
//!   access the network and include the new tap in its constructor. This is
//!   similar to adding a networking card to computer. This way, a machine can
//!   add multiple taps to attach to different networks.

use crate::{control::ControlError, id::Id, Control, Message};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    sync::{broadcast, mpsc, Barrier},
    time::sleep,
};

mod tap;
pub use tap::Tap;
pub(crate) use tap::TapEnvironment;

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;

/// A MAC address that uniquely identifies a [`Tap`] on a network.
pub type Mac = u64;

/// A data transfer rate
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Baud(u64);

impl Baud {
    /// Specify a baud rate in bits per second
    pub fn bits_per_second(rate: u64) -> Self {
        Self(rate / 8)
    }

    /// Specify a baud rate in bytes per second
    pub fn bytes_per_second(rate: u64) -> Self {
        Self(rate)
    }
}

/// A builder for network customization. If a simple network is desired,
/// consider using [`Network::basic()`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct NetworkBuilder {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    throughput: Option<Baud>,
}

impl NetworkBuilder {
    /// Create a new network builder
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the maximum transmission unit
    pub fn mtu(mut self, mtu: Mtu) -> Self {
        self.mtu = Some(mtu);
        self
    }

    /// Set the latency of the network, the amount of time it takes for a
    /// message to reach its destination without contention. Unlike throughput,
    /// latency does not affect the delivery time of other messages on the
    /// network. It only refers to the time an isolated message will spend on
    /// the wire before it is delivered.
    pub fn latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    /// Set the throughput of the network, the amount of data that the network
    /// can transfer in a given time. A low throughput means that if many
    /// messages are sent on the network at the same time, later messages will
    /// be queued for delivery until prior messages have been fully transferred.
    /// Larger messages take longer to transfer than shorter ones.
    pub fn throughput(mut self, throughput: Baud) -> Self {
        self.throughput = Some(throughput);
        self
    }

    /// Create the network with the given settings
    pub fn build(self) -> Arc<Network> {
        Arc::new(Network::new(self.mtu, self.latency, self.throughput))
    }
}

/// A [`Message`] in flight over a network. A delivery includes the information
/// usually included in a data-link frame and thus abstracts over different
/// network technologies.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    /// The message being sent
    pub message: Message,
    /// Identifies the [`Tap`] that sent the message
    pub sender: Mac,
    /// Identifies the [`Tap`] that should receive the message. If the
    /// destination is `None`, the message should be broadcast.
    pub destination: Option<Mac>,
    /// The protocol that should respond to the packet, usually an IP protocol
    pub protocol: Id,
}

/// A network that allows the exchange of [`Message`]s between
/// [`Machine`](crate::Machine)s.
///
/// See the module-level documentation for usage directions. The network
/// included with Elvis aims to be general-purpose. It supports direct
/// exchanges, broadcasting, and customizable latency, throughput, and
/// reliability. It should provide a reasonable approximation of many kinds of
/// networks, including Ethernet and WiFi.
pub struct Network {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    throughput: Option<Baud>,
    /// The sending half of a channel for taps to send messages to the network
    /// for delivery over the network. The other half is `delivery_receiver`.
    delivery_sender: mpsc::Sender<Delivery>,
    /// The receiving half of a channel for receiving messages from taps for
    /// delivery over the network. The other half is `delivery_sender`.
    delivery_receiver: Arc<RwLock<Option<mpsc::Receiver<Delivery>>>>,
    /// A channel for sending a message to all taps attached to the network
    broadcast: broadcast::Sender<Delivery>,
    /// A vector for channels for sending messages to specific taps attached to
    /// the network
    taps: Arc<RwLock<Vec<mpsc::Sender<Delivery>>>>,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl Network {
    /// An identifier for the network type
    pub const ID: Id = Id::from_string("Network");

    /// Create a new network with the given properties
    fn new(mtu: Option<Mtu>, latency: Option<Duration>, throughput: Option<Baud>) -> Self {
        let funnel = mpsc::channel(16);
        Self {
            mtu,
            latency,
            throughput,
            delivery_sender: funnel.0,
            delivery_receiver: Arc::new(RwLock::new(Some(funnel.1))),
            taps: Default::default(),
            broadcast: broadcast::channel::<Delivery>(16).0,
        }
    }

    /// Create a default network with unlimited MTU and throughput and no
    /// latency
    pub fn basic() -> Arc<Self> {
        Arc::new(Default::default())
    }

    /// Get an access point to the network. The returned [`Tap`] can be added to
    /// a [`Pci`](crate::protocols::Pci) to allow a [`Machine`](crate::Machine)
    /// to send and receive messages through the network.
    pub fn tap(self: &Arc<Self>) -> Tap {
        let (send, receive) = mpsc::channel(16);
        let mac = self.taps.read().unwrap().len();
        self.taps.write().unwrap().push(send);
        Tap::new(self.clone(), mac as Mac, receive)
    }

    /// Called at the beginning of the simulation to start the network running
    pub(crate) fn start(self: Arc<Self>, barrier: Arc<Barrier>) {
        let mut receiver = self.delivery_receiver.write().unwrap().take().unwrap();
        let throughput = self.throughput;
        let latency = self.latency;
        let taps = self.taps.clone();
        let broadcast = self.broadcast.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            while let Some(delivery) = receiver.recv().await {
                if let Some(throughput) = throughput {
                    let ms = delivery.message.len() as u64 * 1000 / throughput.0;
                    println!("{}, {}, {}", delivery.message.len(), throughput.0, ms);
                    sleep(Duration::from_millis(ms)).await;
                }

                let taps = taps.clone();
                let broadcast = broadcast.clone();
                tokio::spawn(async move {
                    if let Some(latency) = latency {
                        sleep(latency).await;
                    }
                    match delivery.destination {
                        Some(destination) => {
                            let tap = {
                                let taps = taps.read().unwrap();
                                match taps.get(destination as usize) {
                                    Some(tap) => tap,
                                    None => {
                                        tracing::error!(
                                            "Trying to deliver to an invalid MAC address"
                                        );
                                        return;
                                    }
                                }
                                .clone()
                            };
                            match tap.send(delivery).await {
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::error!("Failed to deliver a message: {}", e)
                                }
                            }
                        }
                        None => match broadcast.clone().send(delivery) {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!("Failed to deliver a message: {}", e)
                            }
                        },
                    }
                });
            }
        });
    }

    /// Set the destination MAC address on a [`Control`]
    pub fn set_destination(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 0), mac);
    }

    /// Get the destination MAC address on a [`Control`]
    pub fn get_destination(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u64()?)
    }

    /// Set the source MAC address on a [`Control`]
    pub fn set_sender(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 1), mac);
    }

    /// Get the source MAC address on a [`Control`]
    pub fn get_sender(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u64()?)
    }

    /// Set the protocol that should respond to a network frame on a [`Control`]
    pub fn set_protocol(protocol: Id, control: &mut Control) {
        control.insert((Self::ID, 2), protocol.into_inner());
    }

    /// Get the protocol that should respond to a network frame on a [`Control`]
    pub fn get_protocol(control: &Control) -> Result<Id, ControlError> {
        Ok(control.get((Self::ID, 2))?.ok_u64()?.into())
    }
}
