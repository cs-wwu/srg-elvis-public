use crate::{
    internet::NetworkHandle,
    machine::ProtocolMap,
    network::{Mac, Network},
    protocol::SharedProtocol,
    Id, Machine, Message,
};
use flume::{Receiver, Sender};
use std::{cell::RefCell, sync::Arc, thread};

thread_local! {
    static GCD_HANDLE: RefCell<Option<GcdHandle>> = Default::default();
    static PROTOCOLS: RefCell<ProtocolMap> = Default::default();
}

#[derive(Debug, Clone)]
pub(crate) struct Gcd {
    tx: Sender<Task>,
    rx: Receiver<Task>,
    threads: usize,
}

impl Gcd {
    pub fn new(threads: usize) -> Self {
        let (tx, rx) = flume::unbounded::<Task>();
        Self { tx, rx, threads }
    }

    pub fn install(&self) {
        let tx = self.tx.clone();
        let threads = self.threads;
        GCD_HANDLE.with(move |handle| {
            *handle.borrow_mut() = Some(GcdHandle { tx, threads });
        });
    }

    pub fn start(self, machines: Arc<Vec<Machine>>, networks: Arc<Vec<Network>>) {
        if self.threads > 1 {
            let mut threads = Vec::with_capacity(self.threads);
            for _ in 0..self.threads {
                let networks = networks.clone();
                let machines = machines.clone();
                let me = self.clone();
                let thread = thread::spawn(move || main_loop(me, networks, machines));
                threads.push(thread);
            }
            for thread in threads {
                thread.join().unwrap();
            }
        } else {
            main_loop(self, networks, machines);
        }
    }
}

fn main_loop(gcd: Gcd, networks: Arc<Vec<Network>>, machines: Arc<Vec<Machine>>) {
    gcd.install();
    let Gcd { tx, rx, threads: _ } = gcd;
    while let Ok(task) = rx.recv() {
        match task {
            Task::Shutdown => break,

            Task::Delivery(delivery) => {
                let network = &networks[delivery.network.0];
                match delivery.destination {
                    Some(destination) => {
                        let machine_handle = network.machines[destination as usize];
                        machines[machine_handle.0].receive(delivery);
                    }
                    None => {
                        for machine_handle in network.machines.iter() {
                            machines[machine_handle.0].receive(delivery.clone());
                        }
                    }
                }
            }

            Task::Once(job, protocols) => {
                set_protocols(protocols);
                job();
            }

            Task::Repeat(job, protocols) => {
                set_protocols(protocols.clone());
                job();
                tx.send(Task::Repeat(job, protocols)).unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
struct GcdHandle {
    pub tx: Sender<Task>,
    pub threads: usize,
}

enum Task {
    Shutdown,
    Delivery(Delivery),
    Once(Box<dyn FnOnce() + Send + 'static>, ProtocolMap),
    Repeat(Box<dyn Fn() + Send + 'static>, ProtocolMap),
}

/// A [`Message`] in flight over a network. A delivery includes the information
/// usually included in a data-link frame and thus abstracts over different
/// network technologies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The network to send on
    pub network: NetworkHandle,
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

fn queue(task: Task) {
    GCD_HANDLE.with(|handle| handle.borrow().as_ref().unwrap().tx.send(task).unwrap())
}

pub fn delivery(delivery: Delivery) {
    queue(Task::Delivery(delivery));
}

pub fn job(job: impl FnOnce() + Send + 'static) {
    queue(Task::Once(Box::new(job), get_protocols()));
}

pub fn job_at(job: impl Fn() + Send + 'static) {
    queue(Task::Repeat(Box::new(job), get_protocols()))
}

pub fn shut_down() {
    GCD_HANDLE.with(|handle| {
        let handle = handle.borrow();
        let handle = handle.as_ref().unwrap();
        for _ in 0..handle.threads {
            handle.tx.send(Task::Shutdown).unwrap();
        }
    })
}

pub(crate) fn set_protocols(protocols: ProtocolMap) {
    PROTOCOLS.with(|handle| handle.replace(protocols));
}

fn get_protocols() -> ProtocolMap {
    PROTOCOLS.with(|handle| handle.borrow().clone())
}

pub fn get_protocol(id: Id) -> Option<SharedProtocol> {
    PROTOCOLS.with(|handle| handle.borrow().protocol(id))
}
