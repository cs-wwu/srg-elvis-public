use crate::{
    core::{network::Attachment, Network},
    protocols::tap::Delivery,
};
use async_trait::async_trait;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

pub struct Unreliable {
    rng: Arc<Mutex<SmallRng>>,
    failure_rate: f64,
}

impl Unreliable {
    pub fn new(failure_rate: f64) -> Self {
        Self {
            rng: Arc::new(Mutex::new(SmallRng::seed_from_u64(0xBAD5EED))),
            failure_rate,
        }
    }
}

#[async_trait]
impl Network for Unreliable {
    async fn send(
        self: Arc<Self>,
        delivery: Delivery,
        attachments: &[Attachment],
    ) -> Result<(), Box<dyn Error>> {
        for attachment in attachments.iter().filter(|attachment| {
            attachment.machine != delivery.sender
                && self.rng.lock().unwrap().gen_bool(self.failure_rate)
        }) {
            attachment.sender.send(delivery.clone()).await.unwrap();
        }
        Ok(())
    }
}
