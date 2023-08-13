//! Generates protocols from parsing data for machines
//! Future protocols can go here for easy import to the machine generator

use core::panic;
use std::collections::HashMap;

use crate::ndl::generating::generator_utils::{ip_string_to_ip, ip_or_name};

use elvis_core::protocols::arp::subnetting::{Ipv4Mask, SubnetInfo};
use elvis_core::protocols::ipv4::Ipv4Address;
use elvis_core::protocols::Arp;

pub fn arp_builder(name_to_ip : &HashMap<String, Ipv4Address>, options: &HashMap<String, String>) -> Arp {
    if options.contains_key("local") {
        assert!(
            options.contains_key("default"),
            "Arp protocol doesn't contain default."
        );
        
        let default = options.get("default").unwrap().to_string();
        let default_gateway = match ip_or_name(default.clone()) {
            true => Ipv4Address::new(ip_string_to_ip(default.clone(), "default arp id")),
            false => {
                match name_to_ip.contains_key(&default) {
                    true => *name_to_ip.get(&default).unwrap(),
                    false => panic!("Invalid name for default arp gateway"),
                }
            },
        };
        Arp::basic().preconfig_subnet(
            Ipv4Address::new(ip_string_to_ip(
                options.get("local").unwrap().to_string(),
                "local arp ip",
            )),
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(32),
                default_gateway,
            },
        )
    } else {
        Arp::basic()
    }
}