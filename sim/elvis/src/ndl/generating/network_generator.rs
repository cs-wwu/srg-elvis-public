//! Generates networks from a given parse

use crate::ndl::generating::generator_utils::ip_string_to_ip;
use crate::ndl::parsing::parsing_data::*;
use elvis_core::{
    network::{Latency, NetworkBuilder},
    protocols::ipv4::Ipv4Address,
};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

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
    // IP_hash contains a hashmap linking ids to vectors of ips
    let mut networks = HashMap::new();
    let mut ip_hash = HashMap::new();

    for (id, net) in n {
        // insert networks into the hashmap
        let network = NetworkBuilder::new();

        for option in &net.options {
            match option.0.as_str() {
                "latency" => {
                    if !net.options.contains_key("latency_var"){
                        network.latency(Latency::constant(Duration::from_secs(
                            option.1.parse::<u64>().unwrap_or_else(|_e| {
                                panic!("Network {}: Invalid latency value passed to network.", id);
                            }),
                        )));
                    }
                }

                "latency_var" => {
                    let mut l = 0;

                    // only if the arguments also
                    if net.options.contains_key("latency") {
                        l = net
                            .options
                            .get("latency")
                            .unwrap()
                            .parse::<u64>()
                            .unwrap_or_else(|_e| {
                                panic!("Network {}: Invalid latency value passed to network.", id);
                            });
                    }

                    let rand = option.1.parse::<u64>().unwrap_or_else(|_e| {
                        panic!(
                            "Network {}: Invalid latency variation value passed to network.",
                            id
                        );
                    });

                    network.latency(Latency::variable(
                        Duration::from_secs(l),
                        Duration::from_millis(rand),
                    ));
                }

                "mtu" => {
                    network.mtu(option.1.parse::<u32>().unwrap_or_else(|_e| {
                        panic!("Network {}: Invalid mtu value passed to network.", id);
                    }));
                }

                "loss" => {
                    network.loss_rate(option.1.parse::<f32>().unwrap_or_else(|_e| {
                        panic!("Network {}: Invalid loss rate passed to network.", id);
                    }));
                }

                "id" => {
                    // do nothing
                }

                _ => {
                    panic!("Network {}: Invalid argument passed. Got {}", id, option.0);
                }
            }
        }

        networks.insert(id.clone(), network.build());

        let mut ip_vec: Vec<Ipv4Address> = Vec::new();
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
                        let end_ip = temp[1].parse::<u8>().unwrap_or_else(|_| {
                            panic!("Network {}: Invalid ending IP range number. Expected <u8> found: {}", id, temp[1])
                        });

                        assert!(
                            end_ip >= start_ip[3],
                            "Network {}: Invalid Cidr format, end IP value ({}) greater than start IP value ({})",
                            id, end_ip, start_ip[3]
                        );

                        while start_ip[3] <= end_ip {
                            assert!(
                                !temp_ips.contains(&start_ip),
                                "Network {id}: Duplicate IP found in range: {start_ip:?}"
                            );

                            ip_vec.push(start_ip.into());
                            temp_ips.insert(start_ip);
                            start_ip[3] += 1;
                        }
                    }
                    "ip" => {
                        let real_ip = ip_string_to_ip(value, &id);
                        assert!(
                            !temp_ips.contains(&real_ip),
                            "Network {id}: Duplicate IP found in IP: {real_ip:?}"
                        );

                        ip_vec.push(real_ip.into());
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
        // let ip_table: IpToNetwork = ips.into_iter().collect();
        // networks.insert(id, (network, ip_table, ip_list));
        ip_hash.insert(id, ip_vec);
    }
    NetworkInfo {
        nets: networks,
        ip_hash,
    }
}
