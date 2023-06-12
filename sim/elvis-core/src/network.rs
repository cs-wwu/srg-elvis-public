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
//! - Add a [`Pci`](crate::protocols::Pci) protocol to the machine that wants to
//!   access the network and place a pointer to a network in its constructor
//!   (using `my_network.clone()`). This is similar to adding a networking card
//!   to a computer. This way, a machine can attach to multiple different networks.
//! - When [`Pci::start`](crate::protocols::Pci) is called,
//!   a new [`PciSession`] will be created for each network in the Pci's constructor.
//!   Also called "taps", these sessions are access points to the network that can be
//!   used to send and receive messages.
//!   Each session also acts as an identifier so that peers on the network can
//!   exchange messages directly.

use crate::{protocols::pci::PciSession, FxDashMap, Message};
use rand::{distributions::Uniform, prelude::Distribution};
use std::{
    any::TypeId,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Notify, time::sleep};

type Taps = Arc<FxDashMap<Mac, Arc<PciSession>>>;

/// A network that allows the exchange of [`Message`]s between
/// [`Machine`](crate::Machine)s.
///
/// See the module-level documentation for usage directions. The network
/// included with Elvis aims to be general-purpose. It supports direct
/// exchanges, broadcasting, and customizable latency, throughput, and
/// reliability. It should provide a reasonable approximation of many kinds of
/// networks, including Ethernet and WiFi.
pub struct Network {
    pub(crate) mtu: Mtu,
    latency: Latency,
    throughput: Throughput,
    loss_rate: f32,
    throughput_permit: Arc<Notify>,
    taps: Taps,
    next_mac: Mutex<Mac>,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(None, Default::default(), Default::default(), 0.0)
    }
}

impl Network {
    /// Create a new network with the given properties
    fn new(mtu: Option<Mtu>, latency: Latency, throughput: Throughput, loss_rate: f32) -> Self {
        let throughput_permit = Arc::new(Notify::new());
        throughput_permit.notify_one();
        Self {
            mtu: mtu.unwrap_or(Mtu::MAX),
            latency,
            throughput,
            loss_rate,
            throughput_permit,
            taps: Default::default(),
            next_mac: Default::default(),
        }
    }

    /// Create a default network with unlimited MTU and throughput and no
    /// latency
    pub fn basic() -> Arc<Self> {
        Arc::new(Default::default())
    }

    pub(crate) fn next_mac(self: &Arc<Self>) -> Mac {
        let mut lock = self.next_mac.lock().unwrap();
        let mac = *lock;
        *lock += 1;
        mac
    }

    pub(crate) fn register_tap(self: &Arc<Self>, mac: Mac, session: Arc<PciSession>) {
        self.taps.insert(mac, session);
    }

    /// Called at the beginning of the simulation to start the network running
    pub(crate) async fn send(self: &Arc<Self>, delivery: Delivery) {
        if self.loss_rate > 0.0 && rand::random::<f32>() < self.loss_rate {
            // Drop the message
            return;
        }

        let throughput = self.throughput.next();
        if throughput.0 > 0 {
            self.throughput_permit.notified().await;
            let ms = delivery.message.len() as u64 * 1000 / throughput.0;
            sleep(Duration::from_millis(ms)).await;
            self.throughput_permit.notify_one();
        }

        let latency = self.latency.next();
        if latency > Duration::ZERO {
            sleep(latency).await;
        }
        match delivery.destination {
            Some(destination) => {
                let tap = {
                    match self.taps.get(&destination) {
                        Some(tap) => tap,
                        None => {
                            tracing::error!("Trying to deliver to an invalid MAC address");
                            return;
                        }
                    }
                    .clone()
                };
                match tap.receive(delivery) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Failed to deliver a message: {}", e)
                    }
                }
            }

            None => {
                for tap in self.taps.iter() {
                    match tap.receive(delivery.clone()) {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to deliver a message: {}", e)
                        }
                    }
                }
            }
        }
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
    pub protocol: TypeId,
}

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u16;

/// A MAC address that uniquely identifies a [`PciSession`] on a network.
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
