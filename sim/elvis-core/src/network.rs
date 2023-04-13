use crate::{control::ControlError, internet::MachineHandle, Control, Id};
use rand::{distributions::Uniform, prelude::Distribution};
use std::time::Duration;

#[derive(Clone, PartialEq)]
pub struct Network {
    pub mtu: Mtu,
    pub latency: Latency,
    pub throughput: Throughput,
    pub loss_rate: f32,
    pub machines: Vec<MachineHandle>,
}

impl Network {
    const ID: Id = Id::from_string("Network");

    /// Create a new network builder
    pub fn new() -> Self {
        Self {
            mtu: Mtu::MAX,
            latency: Default::default(),
            throughput: Default::default(),
            loss_rate: Default::default(),
            machines: Default::default(),
        }
    }

    /// Set the maximum transmission unit
    pub fn mtu(mut self, mtu: Mtu) -> Self {
        self.mtu = mtu;
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

    pub fn mac_for_machine(&self, machine: MachineHandle) -> Mac {
        self.machines
            .iter()
            .enumerate()
            .find(|(_, candidate)| **candidate == machine)
            .expect("This network is not connected to the given machine")
            .0 as Mac
    }

    pub fn connect(&mut self, machine: MachineHandle) -> Mac {
        let mac = self.machines.len() as u64;
        self.machines.push(machine);
        mac
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
