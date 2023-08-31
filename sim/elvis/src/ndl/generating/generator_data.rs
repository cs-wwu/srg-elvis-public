use crate::ip_generator::IpGenerator;
use elvis_core::Network;
use std::{collections::HashMap, sync::Arc};

pub struct NetworkInfo {
    pub nets: HashMap<String, Arc<Network>>,
    pub ip_hash: HashMap<String, IpGenerator>,
}
