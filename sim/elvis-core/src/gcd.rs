use crate::{
    internet::NetworkHandle,
    network::{Mac, Network},
    Id, Machine, Message,
};
use flume::{Receiver, Sender};
use std::{sync::Arc, thread, time::Instant};

#[derive(Debug, Clone)]
pub struct GcdHandle {
    tx: Sender<Task>,
    threads: usize,
}

impl GcdHandle {
    pub fn delivery(&self, delivery: Delivery) {
        self.queue(Task::Delivery(delivery));
    }

    pub fn job(&self, job: impl FnOnce() + Send + 'static) {
        self.queue(Task::Once(Box::new(job)));
    }

    pub fn job_at(&self, job: impl FnOnce() + Send + 'static, when: Instant) {
        self.queue(Task::At(Box::new(job), when))
    }

    fn queue(&self, task: Task) {
        self.tx.send(task).unwrap();
    }

    pub fn shut_down(&self) {
        for _ in 0..self.threads {
            self.queue(Task::Shutdown);
        }
    }
}

pub struct Gcd {
    tx: Sender<Task>,
    rx: Receiver<Task>,
    threads: usize,
}

impl Gcd {
    pub fn new(threads: usize) -> (Self, GcdHandle) {
        let (tx, rx) = flume::unbounded::<Task>();
        (
            Self {
                tx: tx.clone(),
                rx,
                threads,
            },
            GcdHandle { tx, threads },
        )
    }

    pub fn start(self, machines: Arc<Vec<Machine>>, networks: Arc<Vec<Network>>) {
        let mut threads = Vec::with_capacity(self.threads);
        for _ in 0..self.threads {
            let tx = self.tx.clone();
            let rx = self.rx.clone();
            let networks = networks.clone();
            let machines = machines.clone();
            let handle = thread::spawn(move || main_loop(tx, rx, networks, machines));
            threads.push(handle);
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }
}

fn main_loop(
    tx: Sender<Task>,
    rx: Receiver<Task>,
    networks: Arc<Vec<Network>>,
    machines: Arc<Vec<Machine>>,
) {
    while let Ok(task) = rx.recv() {
        match task {
            Task::Shutdown => break,
            Task::Once(job) => job(),
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
            Task::At(job, when) => {
                if Instant::now() < when {
                    // Requeue the job
                    tx.send(Task::At(job, when)).unwrap();
                } else {
                    job();
                }
            }
        }
    }
}

enum Task {
    Shutdown,
    Delivery(Delivery),
    Once(Box<dyn FnOnce() + Send + 'static>),
    At(Box<dyn FnOnce() + Send + 'static>, Instant),
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
