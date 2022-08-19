use async_trait::async_trait;
use elvis_core::{
    network::{Attachment, Delivery},
    Network,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

/// A network with unreliable delivery.
pub struct Unreliable {
    /// A random number generator to determine delivery success
    rng: Arc<Mutex<SmallRng>>,
    /// A number in the range [0, 1] to determine the frequency of successful
    /// delivery
    success_rate: f64,
}

impl Unreliable {
    /// Creates a new instance of the network with the given delivery success
    /// rate in the range [0, 1].
    pub fn new(success_rate: f64) -> Self {
        Self {
            rng: Arc::new(Mutex::new(SmallRng::seed_from_u64(0xBAD5EED))),
            success_rate,
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
        for attachment in attachments {
            if self.rng.lock().unwrap().gen_bool(self.success_rate) {
                // We don't want to unwrap here if a send fails. It might be
                // that the simulation is shutting down and the receiver has
                // closed the channel, which we should handle gracefully.
                match attachment.sender.send(delivery.clone()).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
        Ok(())
    }
}
