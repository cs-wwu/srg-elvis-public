//! Generates networks from a given parse

use crate::ndl::parsing::parsing_data::*;
use crate::{ip_generator::IpGenerator, ndl::generating::generator_utils::ip_string_to_ip};
use elvis_core::protocols::arp::subnetting::Ipv4Net;
use elvis_core::Network;
use std::collections::{HashMap, HashSet};

use super::generator_data::NetworkInfo;

/// Network Generator generates networks from a given [Networks] struct and places them in the [Internet]
/// Returns said networks and corresponding ip tables for later use with machines
pub fn network_generator(n: Networks) -> NetworkInfo {
    // For each network we need
    // let network = internet.network(Reliable::new(1500));
    // Additionally we need each IP to be stored in the IP table with that assocaited network:
    // let ip_table: IpToNetwork = [(IP_ADDRESS_1, network), (IP_ADDRESS_2, network)].into_iter().collect();

    // HashMap(network_1, (Network, Iptable))

    // Networks contains a hashmap linking ids to networks
    // IP_gen_hash contains a hashmap linking ids to IpGenerator with possible IPs
    let mut networks = HashMap::new();
    let mut ip_gen_hash = HashMap::new();

    for (id, net) in n {
        // insert networks into the hashmap
        let network = Network::basic();
        networks.insert(id.clone(), network);

        let mut ip_gen: IpGenerator = IpGenerator::none();
        let mut temp_ips = HashSet::new();
        for ip in net.ip {
            for (option_id, value) in ip.options {
                match option_id.to_ascii_lowercase().as_str() {
                    "range" => {
                        let temp: Vec<&str> = value.split('/').collect();
                        assert_eq!(
                            temp.len(),
                            2,
                            "Network {}: Invalid IP range format, expected 2 values found {}",
                            id,
                            temp.len()
                        );

                        let mut start_ip = ip_string_to_ip(temp[0].to_string(), &id);
                        let end_ip_slice = temp[1].parse::<u8>().unwrap_or_else(|_| {
                            panic!("Network {}: Invalid ending IP range number. Expected <u8> found: {}", id, temp[1])
                        });

                        assert!(
                            end_ip_slice >= start_ip[3],
                            "Network {}: Invalid Cidr format, end IP value ({}) greater than start IP value ({})",
                            id, end_ip_slice, start_ip[3]
                        );

                        while start_ip[3] <= end_ip_slice {
                            assert!(
                                !temp_ips.contains(&start_ip),
                                "Network {id}: Duplicate IP found in range: {start_ip:?}"
                            );

                            ip_gen.return_ip(start_ip.into());
                            temp_ips.insert(start_ip);
                            start_ip[3] += 1;
                        }
                    }
                    "subnet" => {
                        let temp: Vec<&str> = value.split('/').collect();
                        assert_eq!(
                            temp.len(),
                            2,
                            "Network {}: Invalid IP subnet format, expected 2 values found {}",
                            id,
                            temp.len()
                        );

                        let start_ip = ip_string_to_ip(temp[0].to_string(), &id);
                        let mask = temp[1].parse::<u32>().unwrap_or_else(|_| {
                            panic!("Network {}: Invalid ending IP subnet number. Expected <u8> found: {}", id, temp[1])
                        });
                        assert!(mask <= 32, "Invalid mask value");
                        let net = Ipv4Net::new_short(start_ip, mask.into());
                        assert!(
                            ip_gen.is_available(net),
                            "Network {}: Duplicate ip sound in subnet: {:?}",
                            id,
                            net
                        );
                        ip_gen.return_subnet(net);
                    }
                    "ip" => {
                        let real_ip = ip_string_to_ip(value, &id);
                        assert!(
                            !temp_ips.contains(&real_ip),
                            "Network {id}: Duplicate IP found in IP: {real_ip:?}"
                        );

                        ip_gen.return_ip(real_ip.into());
                        temp_ips.insert(real_ip);
                    }
                    _ => {
                        panic!(
                            "Network {}: Invalid network argument provided. Found: {}",
                            id,
                            option_id.to_ascii_lowercase().as_str()
                        )
                    }
                }
            }
        }

        ip_gen_hash.insert(id, ip_gen);
    }
    NetworkInfo {
        nets: networks,
        ip_hash: ip_gen_hash,
    }
}
