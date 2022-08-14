use crate::protocols::tap::Delivery;
use std::{error::Error, sync::Arc};

pub trait Network {
    fn send(self: Arc<Self>, delivery: Delivery) -> Result<(), Box<dyn Error>>;
}
