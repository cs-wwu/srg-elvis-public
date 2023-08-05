use elvis_core::{
    protocols::{arp::subnetting::Ipv4Net, ipv4::Ipv4Address},
    Network,
};
use std::{collections::HashMap, sync::Arc};

use crate::ip_generator::IpGenerator;

pub struct NetworkInfo {
    pub nets: HashMap<String, Arc<Network>>,
    pub ip_hash: HashMap<String, Vec<Ipv4Address>>,
}

#[allow(dead_code)]
pub struct MultiIpGenerator {
    ip_gen: IpGenerator,
}

#[allow(dead_code)]
impl MultiIpGenerator {
    pub fn new() -> MultiIpGenerator {
        let mut ip_gen = IpGenerator::all();
        let all_ips = Ipv4Net::new_short([0, 0, 0, 0], 0);
        ip_gen.block_subnet(all_ips);
        return MultiIpGenerator { ip_gen }
    }

    pub fn add_ip(&mut self, ip: Ipv4Address) {
        self.ip_gen
            .return_subnet(Ipv4Net::new_short(ip, 32).into());
    }

    pub fn add_range(&mut self, net: Ipv4Net) {
        self.ip_gen.return_subnet(net);
    }

    pub fn fetch_ip(&mut self, ip: Ipv4Address) -> Option<Ipv4Address>{
        self.ip_gen.fetch_specific_ip(ip)
    }

    pub fn block_ip(&mut self, ip: Ipv4Address) {
        self.ip_gen.block_subnet(Ipv4Net::new_short(ip, 32));
    }

    pub fn is_available(&self, net: Ipv4Net) -> bool {
        self.ip_gen.is_available(net)
    }
}
