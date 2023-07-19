use elvis_core::{protocols::ipv4::Ipv4Address, Network};
use std::{collections::HashMap, sync::Arc};

pub struct NetworkInfo {
    pub nets: HashMap<String, Arc<Network>>,
    pub ip_hash: HashMap<String, Vec<Ipv4Address>>,
}
