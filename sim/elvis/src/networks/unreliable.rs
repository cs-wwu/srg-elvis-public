use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Sender};

pub struct Unreliable {
    rng: Arc<Mutex<SmallRng>>,
    success_rate: f64,
}

impl Unreliable {
    pub fn new(success_rate: f64) -> Self {
        Self {
            rng: Arc::new(Mutex::new(SmallRng::seed_from_u64(0xBAD5EED))),
            success_rate,
        }
    }
}

impl Network for Unreliable {
    fn start(self: Box<Self>, attachments: Arc<[Attachment]>) -> Sender<Delivery> {
        let (sender, mut receiver) = mpsc::channel::<Delivery>(16);
        tokio::spawn(async move {
            while let Some(delivery) = receiver.recv().await {
                for attachment in attachments
                    .iter()
                    .filter(|attachment| attachment.machine != delivery.sender)
                {
                    if self.rng.lock().unwrap().gen_bool(self.success_rate) {
                        attachment.sender.send(delivery.clone()).await.unwrap();
                    }
                }
            }
        });
        sender
    }
}
