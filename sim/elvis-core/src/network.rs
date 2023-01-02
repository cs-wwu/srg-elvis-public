use crate::{
    control::{ControlError, Key, Primitive},
    id::Id,
    machine::ProtocolMap,
    protocol::Context,
    session::{QueryError, SendError, SharedSession},
    Control, Message,
};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, oneshot, Barrier,
    },
    time::{sleep, timeout},
};

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;
pub type Mac = u64;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Baud(u32);

impl Baud {
    pub fn bits_per_second(rate: u32) -> Self {
        Self(rate / 8)
    }

    pub fn bytes_per_second(rate: u32) -> Self {
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

struct QueuedDelivery {
    bytes: u32,
    notify: oneshot::Sender<()>,
}

pub struct Network {
    mtu: Option<Mtu>,
    latency: Option<Duration>,
    throughput: Option<Baud>,
    broadcast: broadcast::Sender<Message>,
    connections: Arc<RwLock<Vec<mpsc::Sender<Message>>>>,
    started: Arc<RwLock<bool>>,
    send_queue_sender: mpsc::Sender<QueuedDelivery>,
    send_queue_receiver: Arc<RwLock<Option<mpsc::Receiver<QueuedDelivery>>>>,
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
        let send_queue = mpsc::channel(16);
        Self {
            mtu,
            latency,
            throughput,
            connections: Default::default(),
            broadcast: broadcast::channel::<Message>(16).0,
            started: Default::default(),
            send_queue_sender: send_queue.0,
            send_queue_receiver: Arc::new(RwLock::new(Some(send_queue.1))),
        }
    }

    pub fn basic() -> Arc<Self> {
        Arc::new(Default::default())
    }

    pub fn tap(self: &Arc<Self>) -> Tap {
        let (send, receive) = mpsc::channel(16);
        self.connections.write().unwrap().push(send);
        Tap::new(self.clone(), receive)
    }

    // TODO(hardint): Barrier needed
    fn start(&self) {
        let started = {
            let mut lock = self.started.write().unwrap();
            let started = *lock;
            *lock = true;
            started
        };
        if let Some(throughput) = self.throughput {
            if !started {
                let mut receiver = self.send_queue_receiver.write().unwrap().take().unwrap();
                tokio::spawn(async move {
                    let mut queue = VecDeque::<QueuedDelivery>::new();
                    loop {
                        let next = queue.back();
                        let delay = match next {
                            Some(next) => Duration::from_millis(
                                next.bytes as u64 * 1000 / throughput.0 as u64,
                            ),
                            None => Duration::MAX,
                        };
                        let now = SystemTime::now();
                        match timeout(delay, receiver.recv()).await {
                            Ok(delivery) => {
                                match delivery {
                                    Some(delivery) => {
                                        queue.push_front(delivery);
                                        let elapsed = now
                                            .elapsed()
                                            .unwrap()
                                            .as_millis()
                                            .try_into()
                                            .unwrap_or(u32::MAX);
                                        queue.back_mut().unwrap().bytes -=
                                            elapsed * throughput.0 / 1000;
                                    }
                                    // The other side was closed? Check docs.
                                    // This is probably a simulation shutting
                                    // down situation.
                                    None => break,
                                }
                            }
                            Err(_) => {
                                // Safe to unwrap here because we got a timeout,
                                // which means that queue.back() was Some
                                // earlier
                                let delivery = queue.pop_back().unwrap();
                                match delivery.notify.send(()) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to signal a message delivery: {:?}",
                                            e
                                        )
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    pub fn set_destination_mac(mac: Mac, control: &mut Control) {
        control.insert((Self::ID, 0), mac);
    }

    pub fn get_destination_mac(control: &Control) -> Result<Mac, ControlError> {
        Ok(control.get((Self::ID, 0))?.ok_u64()?)
    }
}

pub struct Tap {
    network: Arc<Network>,
    direct_receiver: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
}

impl Tap {
    pub fn new(network: Arc<Network>, receiver: mpsc::Receiver<Message>) -> Self {
        Self {
            network,
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

    pub(crate) fn send(&self, message: Message, control: Control) -> Result<(), SendError> {
        if let Some(mtu) = self.network.mtu {
            if message.len() > mtu as usize {
                Err(SendError::Mtu(mtu))?
            }
        }

        let latency = self.network.latency;
        match Network::get_destination_mac(&control) {
            Ok(destination) => {
                let destination = destination as usize;
                let channel = self
                    .network
                    .connections
                    .read()
                    .unwrap()
                    .get(destination)
                    .ok_or_else(|| {
                        tracing::error!("The destination mac is out of bounds: {}", destination);
                        SendError::MissingContext
                    })?
                    .clone();
                tokio::spawn(async move {
                    if let Some(latency) = latency {
                        sleep(latency).await;
                    }
                    match channel.clone().send(message).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to send on direct network: {}", e);
                        }
                    }
                });
                Ok(())
            }

            Err(_) => {
                let broadcast = self.network.broadcast.clone();
                tokio::spawn(async move {
                    if let Some(latency) = latency {
                        sleep(latency).await;
                    }
                    match broadcast.send(message) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            tracing::error!("Failed to send on broadcast network: {}", e);
                            Err(SendError::Other)
                        }
                    }
                });
                Ok(())
            }
        }
    }

    pub(crate) fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        match key {
            Network::MTU_QUERY_KEY => Ok(self.network.mtu.unwrap_or(0).into()),
            _ => Err(QueryError::MissingKey),
        }
    }
}

fn receive_direct(message: Option<Message>, environment: TapEnvironment) {
    if let Some(message) = message {
        match environment
            .session
            .clone()
            .receive(message, environment.context())
        {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to receive on direct network: {}", e);
            }
        }
    }
}

fn receive_broadcast(message: Result<Message, RecvError>, environment: TapEnvironment) {
    match message {
        Ok(message) => {
            let context = Context::new(environment.protocols);
            match environment.session.receive(message, context) {
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
    pub session: SharedSession,
}

impl TapEnvironment {
    pub fn new(protocols: ProtocolMap, session: SharedSession) -> Self {
        Self { protocols, session }
    }

    pub fn context(&self) -> Context {
        Context::new(self.protocols.clone())
    }
}
