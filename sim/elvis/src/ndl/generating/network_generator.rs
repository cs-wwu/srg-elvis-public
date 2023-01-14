//! Generates networks from a given parse

use std::collections::{HashMap, HashSet};
use crate::{ndl::generating::generator_utils::{ip_string_to_ip}};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::{
    internet::{NetworkHandle},
    protocols::ipv4::{IpToNetwork},
    Internet, networks::Reliable,
};

/// Network Generator generates networks from a given [Networks] struct and places them in the [Internet]
/// Returns said networks and corresponding ip tables for later use with machines
pub fn network_generator(
    n: Networks,
    internet: &mut Internet
) -> Result<HashMap<String, (NetworkHandle, IpToNetwork, HashSet<[u8; 4]>)>, String> {
    // For each network we need
    // let network = internet.network(Reliable::new(1500));
    // Additionally we need each IP to be stored in the IP table with that assocaited network:
    // let ip_table: IpToNetwork = [(IP_ADDRESS_1, network), (IP_ADDRESS_2, network)].into_iter().collect();

    // HashMap(network_1, (Network, Iptable))
    let mut networks = HashMap::new();
    for (id, net) in n {
        let network = internet.network(Reliable::new(1500));
        let mut ips = Vec::new();
        let mut ip_list = HashSet::new();

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
                        let end_ip = temp[1]
                            .parse::<u8>()
                            .unwrap_or_else(|_| panic!("Network {}: Invalid ending IP range number", id));

                        assert!(
                            end_ip >= start_ip[3],
                            "Network {}: Invalid Cidr format, end IP greater than start IP",
                            id
                        );

                        while start_ip[3] <= end_ip {
                            assert!(
                                !ip_list.contains(&start_ip),
                                "Network {}: Duplicate IP found: {:?}",
                                id,
                                start_ip
                            );

                            ip_list.insert(start_ip);
                            ips.push((start_ip.into(), network));
                            start_ip[3] += 1;
                        }
                    }
                    "ip" => {
                        let real_ip = ip_string_to_ip(value, &id);
                        assert!(
                            !ip_list.contains(&real_ip),
                            "Network {}: Duplicate IP found: {:?}",
                            id,
                            real_ip
                        );

                        ip_list.insert(real_ip);
                        ips.push((real_ip.into(), network));
                    }
                    _ => {
                        return Err(format!(
                            "Network {}: Invalid argument provided '{}'",
                            id, option_id
                        ));
                    }
                }
            }
        }
        let ip_table: IpToNetwork = ips.into_iter().collect();
        networks.insert(id, (network, ip_table, ip_list));
    }
    Ok(networks)
}
