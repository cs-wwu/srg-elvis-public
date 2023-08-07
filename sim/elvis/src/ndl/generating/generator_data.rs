use elvis_core::Network;
use std::{collections::HashMap, sync::Arc};
use crate::ip_generator::IpGenerator;

pub struct NetworkInfo {
    pub nets: HashMap<String, Arc<Network>>,
    pub ip_hash: HashMap<String, IpGenerator>,
}