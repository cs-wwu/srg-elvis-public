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

use crate::{control::ControlError, id::Id, Control, Message, Shutdown};
use rand::{distributions::Uniform, prelude::Distribution};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    sync::{broadcast, mpsc, Barrier},
    time::sleep,
};

type Taps = Arc<RwLock<Vec<mpsc::Sender<Delivery>>>>;

/// A network that allows the exchange of [`Message`]s between
/// [`Machine`](crate::Machine)s.
///
/// See the module-level documentation for usage directions. The network
/// included with Elvis aims to be general-purpose. It supports direct
/// exchanges, broadcasting, and customizable latency, throughput, and
/// reliability. It should provide a reasonable approximation of many kinds of
/// networks, including Ethernet and WiFi.
pub struct Network {
    /// A channel for sending a message to all taps attached to the network.
    /// Each tap subscribes to this.
    broadcast: broadcast::Sender<Delivery>,
    mtu: Mtu,
    latency: Latency,
    throughput: Throughput,
    loss_rate: f32,
    /// The sending half of a channel for taps to send messages to the network
    /// for delivery over the network. The other half is `delivery_receiver`.
    /// Each tap gets its own copy of this and the network does not use it.
    delivery_sender: mpsc::Sender<Delivery>,
    /// The receiving half of a channel for receiving messages from taps for
    /// delivery over the network. The other half is `delivery_sender`.
    delivery_receiver: RwLock<Option<mpsc::Receiver<Delivery>>>,
    /// A vector for channels for sending messages to specific taps attached to
    /// the network
    taps: Taps,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(None, Default::default(), Default::default(), 0.0)
    }
}

impl Network {
    /// An identifier for the network type
    pub const ID: Id = Id::from_string("Network");

    /// Create a new network with the given properties
    fn new(mtu: Option<Mtu>, latency: Latency, throughput: Throughput, loss_rate: f32) -> Self {
        let funnel = mpsc::channel(16);
        Self {
            mtu: mtu.unwrap_or(Mtu::MAX),
            latency,
            throughput,
            loss_rate,
            delivery_sender: funnel.0,
            delivery_receiver: RwLock::new(Some(funnel.1)),
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
        let mac = self.taps.read().unwrap().len() as u64;
        self.taps.write().unwrap().push(send);
        Tap {
            mtu: self.mtu,
            mac,
            delivery_sender: self.delivery_sender.clone(),
            unicast_receiver: RwLock::new(Some(receive)),
            broadcast: RwLock::new(Some(self.broadcast.subscribe())),
        }
    }

    /// Called at the beginning of the simulation to start the network running
    pub(crate) fn start(self: Arc<Self>, shutdown: Shutdown, initialized: Arc<Barrier>) {
        let mut receiver = self.delivery_receiver.write().unwrap().take().unwrap();
        let throughput = self.throughput;
        let latency = self.latency;
        let taps = self.taps.clone();
        let broadcast = self.broadcast.clone();
        tokio::spawn(async move {
            initialized.wait().await;
            let mut shutdown_receiver = shutdown.receiver();
            loop {
                let delivery = tokio::select! {
                    delivery = receiver.recv() => delivery,
                    _ = shutdown_receiver.recv() => {
                        break;
                    }
                };

                let delivery = if let Some(delivery) = delivery {
                    delivery
                } else {
                    break;
                };

                if self.loss_rate > 0.0 && rand::random::<f32>() < self.loss_rate {
                    // Drop the message
                    continue;
                }

                let throughput = throughput.next();
                if throughput.0 > 0 {
                    let ms = delivery.message.len() as u64 * 1000 / throughput.0;
                    tokio::select! {
                        _ = sleep(Duration::from_millis(ms)) => {},
                        _ = shutdown_receiver.recv() => break,
                    };
                    unreachable!();
                }

                let taps = taps.clone();
                let broadcast = broadcast.clone();
                let latency = latency.next();
                if latency > Duration::ZERO {
                    let shutdown = shutdown.clone();
                    let mut shutdown_receiver = shutdown.receiver();
                    tokio::spawn(async move {
                        tokio::select! {
                            _ = sleep(latency) => {},
                            _ = shutdown_receiver.recv() => return,
                        }
                        complete_delivery(delivery, taps, broadcast).await;
                    });
                    unreachable!();
                } else {
                    complete_delivery(delivery, taps, broadcast).await;
                }
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

/// A builder for network customization. If a simple network is desired,
/// consider using [`Network::basic()`].
#[derive(Default, Clone, Copy, PartialEq)]
pub struct NetworkBuilder {
    mtu: Option<Mtu>,
    latency: Latency,
    throughput: Throughput,
    loss_rate: f32,
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
    pub fn latency(mut self, latency: Latency) -> Self {
        self.latency = latency;
        self
    }

    /// Set the throughput of the network, the amount of data that the network
    /// can transfer in a given time. A low throughput means that if many
    /// messages are sent on the network at the same time, later messages will
    /// be queued for delivery until prior messages have been fully transferred.
    /// Larger messages take longer to transfer than shorter ones.
    pub fn throughput(mut self, throughput: Throughput) -> Self {
        self.throughput = throughput;
        self
    }

    /// The percentage of packets that are lost in transmission. Should be given
    /// in the range \[0,1\].
    pub fn loss_rate(mut self, loss_rate: f32) -> Self {
        self.loss_rate = loss_rate;
        self
    }

    /// Create the network with the given settings
    pub fn build(self) -> Arc<Network> {
        Arc::new(Network::new(
            self.mtu,
            self.latency,
            self.throughput,
            self.loss_rate,
        ))
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

/// An access point to a [`Network`]. A tap can be created by calling
/// [`Network::tap`]. Taps should be added to a [`crate::protocols::Pci`]
/// protocol to allow a [`Machine`](crate::Machine) to access the network.
pub struct Tap {
    pub(crate) mtu: Mtu,
    pub(crate) mac: Mac,
    pub(crate) delivery_sender: mpsc::Sender<Delivery>,
    pub(crate) broadcast: RwLock<Option<broadcast::Receiver<Delivery>>>,
    pub(crate) unicast_receiver: RwLock<Option<mpsc::Receiver<Delivery>>>,
}

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;

/// A MAC address that uniquely identifies a [`Tap`] on a network.
pub type Mac = u64;

/// A data transfer rate
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Baud(u64); // Inner value is given in bytes per second

impl Baud {
    pub const MAX: Self = Baud(u64::MAX);
    pub const ZERO: Self = Baud(0);

    /// Specify a baud rate in bits per second
    pub fn bits_per_second(rate: u64) -> Self {
        Self(rate / 8)
    }

    /// Specify a baud rate in bytes per second
    pub fn bytes_per_second(rate: u64) -> Self {
        Self(rate)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Latency {
    base: Duration,
    randomness: Duration,
}

impl Latency {
    pub fn constant(latency: Duration) -> Self {
        Self {
            base: latency,
            randomness: Duration::ZERO,
        }
    }

    pub fn variable(latency: Duration, randomness: Duration) -> Self {
        Self {
            base: latency,
            randomness,
        }
    }

    pub fn next(&self) -> Duration {
        self.base + self.randomness.mul_f32(rand::random())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Throughput {
    base: Baud,
    randomness: Baud,
}

impl Throughput {
    pub fn constant(throughput: Baud) -> Self {
        Self {
            base: throughput,
            randomness: Baud::ZERO,
        }
    }

    pub fn variable(throughput: Baud, randomness: Baud) -> Self {
        Self {
            base: throughput,
            randomness,
        }
    }

    pub fn next(&self) -> Baud {
        if self.randomness.0 == 0 {
            self.base
        } else {
            let uniform = Uniform::from(self.base.0..self.base.0 + self.randomness.0);
            Baud::bytes_per_second(uniform.sample(&mut rand::thread_rng()))
        }
    }
}

async fn complete_delivery(delivery: Delivery, taps: Taps, broadcast: broadcast::Sender<Delivery>) {
    match delivery.destination {
        Some(destination) => {
            let tap = {
                let taps = taps.read().unwrap();
                match taps.get(destination as usize) {
                    Some(tap) => tap,
                    None => {
                        tracing::error!("Trying to deliver to an invalid MAC address");
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
}
