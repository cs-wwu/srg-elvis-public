//! Provides fix-sized buffers to represent the ports of a switch
//! This is to allow the simulation of network congestion.

use crate::{
    network::Delivery,
    Message,
    protocols::udp::Udp,
};

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

    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn size(&self) -> usize {
        self.total
    }

    pub(crate) fn put(&mut self, delivery: &Delivery) -> PutStat {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    fn sample_delivery(message: &Message) -> Delivery {
        Delivery {
            message: message.clone(),
            sender: 0x0,
            destination: None,
            protocol: TypeId::of::<Udp>(),
        }
    }

    #[test]
    fn queue_empty() {
        let queue = DeliveryQueue::new(16);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn queue_on_off() {
        let mut queue = DeliveryQueue::new(16);
        let message = Message::new(b"message");
        let delivery = sample_delivery(&message);

        match queue.put(&delivery) {
            PutStat::DROPPED => panic!("queue dropped packet it should have kept!"),
            PutStat::OK => (),
        };

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.size(), message.len());

        let retrieved = queue.get();
        assert_eq!(delivery, retrieved);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn queue_in_order() {
        let mut queue = DeliveryQueue::new(100);
        let msg1 = Message::new(b"lorem ipsum");
        let del1 = sample_delivery(&msg1);

        let msg2 = Message::new(b"dolor sit amet");
        let del2 = sample_delivery(&msg2);

        match queue.put(&del1) {
            PutStat::DROPPED => panic!("queue dropped packet it should have kept!"),
            PutStat::OK => (),
        };

        match queue.put(&del2) {
            PutStat::DROPPED => panic!("queue dropped packet it should have kept!"),
            PutStat::OK => (),
        };

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.size(), msg1.len() + msg2.len());
        
        let retrieved = queue.get();
        assert_eq!(del1, retrieved);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.size(), msg2.len());

        let retrieved = queue.get();
        assert_eq!(del2, retrieved);
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_drops_packets() {
        let mut queue = DeliveryQueue::new(10);
        let msg1 = Message::new(b"heyyy");
        let del1 = sample_delivery(&msg1);

        let msg2 = Message::new(b"haiiii!!!!!!!!!!");
        let del2 = sample_delivery(&msg2);

        match queue.put(&del1) {
            PutStat::DROPPED => panic!("queue dropped packet it should have kept!"),
            PutStat::OK => (),
        };

        match queue.put(&del2) {
            PutStat::DROPPED => (),
            PutStat::OK => panic!("queue kept packet it should have dropped!"),
        };

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.size(), msg1.len());

        let retrieved = queue.get();
        assert_eq!(del1, retrieved);
        assert!(queue.is_empty());
    }
}

