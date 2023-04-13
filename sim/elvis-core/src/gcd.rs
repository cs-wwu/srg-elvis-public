use crate::{
    internet::NetworkHandle,
    network::{Mac, Network},
    Id, Machine, Message,
};
use flume::{Receiver, Sender};
use std::{sync::Arc, thread};

#[derive(Debug, Clone)]
pub struct GcdHandle {
    tx: Sender<Task>,
}

impl GcdHandle {
    pub fn delivery(&self, delivery: Delivery) {
        self.queue(Task::Delivery(delivery));
    }

    pub fn job(&self, job: impl FnOnce() + Send + 'static) {
        self.queue(Task::Once(Box::new(job)));
    }

    fn queue(&self, task: Task) {
        self.tx.send(task).unwrap();
    }

    pub fn shut_down(&self) {
        for _ in 0..num_cpus::get() {
            self.queue(Task::Shutdown);
        }
    }
}

pub struct Gcd {
    rx: Receiver<Task>,
}

impl Gcd {
    pub fn new() -> (Self, GcdHandle) {
        let (tx, rx) = flume::unbounded::<Task>();
        (Self { rx }, GcdHandle { tx })
    }

    pub fn start(self, machines: Arc<Vec<Machine>>, networks: Arc<Vec<Network>>) {
        let cpus = num_cpus::get();
        let mut threads = Vec::with_capacity(cpus);
        for _ in 0..cpus {
            let rx = self.rx.clone();
            let networks = networks.clone();
            let machines = machines.clone();
            let handle = thread::spawn(move || main_loop(rx, networks, machines));
            threads.push(handle);
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }
}

fn main_loop(rx: Receiver<Task>, networks: Arc<Vec<Network>>, machines: Arc<Vec<Machine>>) {
    while let Ok(task) = rx.recv() {
        match task {
            Task::Shutdown => break,
            Task::Once(func) => func(),
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
        }
    }
}

enum Task {
    Shutdown,
    Delivery(Delivery),
    Once(Box<dyn FnOnce() + Send + 'static>),
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
