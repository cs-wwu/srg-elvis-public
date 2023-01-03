use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    protocol::Context,
    protocols::pci::PciSession,
    session::{QueryError, SendError},
    Control, Message,
};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, Barrier,
    },
    time::sleep,
};

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;
pub type Mac = u64;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Baud(u64);

impl Baud {
    pub fn bits_per_second(rate: u64) -> Self {
        Self(rate / 8)
    }

    pub fn bytes_per_second(rate: u64) -> Self {
        Self(rate)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct NetworkBuilder {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    throughput: Option<Baud>,
}

impl NetworkBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mtu(mut self, mtu: Mtu) -> Self {
        self.mtu = Some(mtu);
        self
    }

    pub fn latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    pub fn throughput(mut self, throughput: Baud) -> Self {
        self.throughput = Some(throughput);
        self
    }

    pub fn build(self) -> Arc<Network> {
        Arc::new(Network::new(self.mtu, self.latency, self.throughput))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    pub message: Message,
    pub sender: Mac,
    pub destination: Option<Mac>,
    pub protocol: Id,
}

pub struct Network {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    throughput: Option<Baud>,
    funnel: (
        mpsc::Sender<Delivery>,
        Arc<RwLock<Option<mpsc::Receiver<Delivery>>>>,
    ),
    broadcast: broadcast::Sender<Delivery>,
    taps: Arc<RwLock<Vec<mpsc::Sender<Delivery>>>>,
}

impl Default for Network {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl Network {
    pub const ID: Id = Id::from_string("Network");
    pub const MTU_QUERY_KEY: Key = (Self::ID, 0);

    fn new(mtu: Option<Mtu>, latency: Option<Duration>, throughput: Option<Baud>) -> Self {
        let funnel = mpsc::channel(16);
        Self {
            mtu,
            latency,
            throughput,
            funnel: (funnel.0, Arc::new(RwLock::new(Some(funnel.1)))),
            taps: Default::default(),
            broadcast: broadcast::channel::<Delivery>(16).0,
        }
    }

    pub fn basic() -> Arc<Self> {
        Arc::new(Default::default())
    }

    pub fn tap(self: &Arc<Self>) -> Tap {
        let (send, receive) = mpsc::channel(16);
        let mac = self.taps.read().unwrap().len();
        self.taps.write().unwrap().push(send);
        Tap::new(self.clone(), mac as Mac, receive)
    }

    pub(crate) fn start(self: Arc<Self>, barrier: Arc<Barrier>) {
        let mut receiver = self.funnel.1.write().unwrap().take().unwrap();
        let throughput = self.throughput;
        let latency = self.latency;
        let taps = self.taps.clone();
        let broadcast = self.broadcast.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            while let Some(delivery) = receiver.recv().await {
                if let Some(throughput) = throughput {
                    sleep(Duration::from_millis(
                        delivery.message.len() as u64 * 1000 / throughput.0,
                    ))
                    .await;
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

    pub fn set_destination(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 0), mac);
    }

    pub fn get_destination(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u64()?)
    }

    pub fn set_sender(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 1), mac);
    }

    pub fn get_sender(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 1))?.ok_u64()?)
    }

    pub fn set_protocol(protocol: Id, control: &mut Control) {
        control.insert((Self::ID, 2), protocol.into_inner());
    }

    pub fn get_protocol(control: &Control) -> Result<Id, ControlError> {
        Ok(control.get((Self::ID, 2))?.ok_u64()?.into())
    }
}

pub struct Tap {
    network: Arc<Network>,
    mac: Mac,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Delivery>>>>,
}

impl Tap {
    fn new(network: Arc<Network>, mac: Mac, receiver: mpsc::Receiver<Delivery>) -> Self {
        Self {
            network,
            mac,
            direct_receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }

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
        let funnel = self.network.funnel.0.clone();
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

    pub(crate) fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Network::MTU_QUERY_KEY => Ok(self.network.mtu.unwrap_or(0).into()),
            _ => Err(QueryError::MissingKey),
        }
    }
}

fn receive_direct(delivery: Option<Delivery>, environment: TapEnvironment) {
    if let Some(delivery) = delivery {
        let context = environment.context();
        match environment.session.clone().receive_pci(delivery, context) {
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

#[derive(Clone)]
pub struct TapEnvironment {
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
