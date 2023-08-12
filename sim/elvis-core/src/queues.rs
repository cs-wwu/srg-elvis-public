//! Provides fix-sized buffers to represent the ports of a switch
//! This is to allow the simulation of network congestion.

use crate::network::Delivery;
use std::sync::{ Arc, Mutex };

pub(crate) enum PutStat { OK, DROPPED }

/// A port, representing an input queue and an output queue
/// in communiction with a particular machine.
pub(crate) struct Port {
    in_queue: Arc<Mutex<DeliveryQueue>>,
    out_queue: Arc<Mutex<DeliveryQueue>>,
}

/// A queue for handling messages
pub(crate) struct DeliveryQueue {
    queue: Vec<Delivery>,
    capacity: usize,
    total: usize,
}

impl DeliveryQueue {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            queue: Vec::new(),
            capacity: capacity,
            total: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub(crate) fn put(&mut self, delivery: Delivery) -> PutStat {
        if self.total + delivery.message.len() > self.capacity {
            PutStat::DROPPED
        } else {
            self.queue.push(delivery.clone());
            self.total += delivery.message.len();

            PutStat::OK
        }
    }
    
    pub(crate) fn get(&mut self) -> Delivery {
        let top = self.queue.swap_remove(0);
        self.total -= top.message.len();

        top
    }
}

