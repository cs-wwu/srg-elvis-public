use std::collections::{HashMap, HashSet};
use crate::{applications::{Capture, SendMessage}};
use super::parsing_data::*;
use elvis_core::{
    internet::{NetworkHandle},
    networks::Reliable,
    protocol::SharedProtocol,
    protocols::ipv4::{IpToNetwork},
    Internet,
    message::Message,
    protocols::{
        ipv4::{ Ipv4},
        udp::Udp,
    },
};

pub async fn core_generator(s: Sim) {
    println!("{:?}", s);
    let mut internet = Internet::new();
    // println!("{:?}", network_generator(s.networks, internet));
    let networks = network_generator(s.networks, &mut internet).unwrap();
    // println!("{:?}", networks);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(networks.get("1").unwrap().1.clone()),
            SendMessage::new_shared("Hello!", networks.get("1").unwrap().2.get(&[12, 34, 56, 89]).unwrap().to_owned().into(),  0xbeef),
        ],
        [networks.get("1").unwrap().0],
    );

    let capture = Capture::new_shared(networks.get("1").unwrap().2.get(&[12, 34, 56, 89]).unwrap().to_owned().into(), 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(networks.get("1").unwrap().1.clone()),
            capture.clone(),
        ],
        [networks.get("1").unwrap().0],
    );

    internet.run().await;
    assert_eq!(
        capture.application().message(),
        Some(Message::new("Hello!"))
    );
}

pub fn network_generator(
    n: Networks,
    internet: &mut Internet,
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
                        let temp: Vec<&str> = value.split("/").collect();
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
                            .expect(&format!("Network {}: Invalid ending IP range number", id));

                        assert!(
                            end_ip >= start_ip[3],
                            "Network {}: Invalid Cidr format, end IP greater than start IP",
                            id
                        );

                        while start_ip[3] <= end_ip {
                            // TODO: Should this be an error or just a skip
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

fn ip_string_to_ip(s: String, net_id: &str) -> [u8; 4] {
    let temp: Vec<&str> = s.split(".").collect();
    let mut new_ip: Vec<u8> = Vec::new();
    for val in temp {
        new_ip.push(val.parse::<u8>().expect(&format!(
            "Network {}: Invalid IP octet (expecting a u8)",
            net_id
        )));
    }

    assert_eq!(
        new_ip.len(),
        4,
        "Network {}: Invalid IP octect count, expected 4 octets found {} octets",
        net_id,
        new_ip.len()
    );

    [new_ip[0], new_ip[1], new_ip[2], new_ip[3]]
}
