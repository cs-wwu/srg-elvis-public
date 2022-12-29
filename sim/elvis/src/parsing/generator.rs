use std::collections::{HashMap, HashSet};

use super::parsing_data::*;
use elvis_core::{Internet, internet::{self, NetworkHandle}, networks::Reliable, protocols::ipv4::{Ipv4Address, IpToNetwork}};


pub async fn core_generator(s: Sim){
    println!("{:?}", s);
    let mut internet = Internet::new();
    println!("{:?}", network_generator(s.networks, internet));
}


pub fn network_generator(n: Networks, mut internet: Internet) -> Result<HashMap<String, (NetworkHandle, IpToNetwork)>, String> {
    // For each network we need
    // let network = internet.network(Reliable::new(1500));
    // Additionally we need each IP to be stored in the IP table with that assocaited network:
    // let ip_table: IpToNetwork = [(IP_ADDRESS_1, network), (IP_ADDRESS_2, network)].into_iter().collect();

    // HashMap(network_1, (Network, Iptable))
    let mut networks = HashMap::new();
    for (id, net) in n{
        let network = internet.network(Reliable::new(1500));
        let mut ips = Vec::new();
        let mut ip_list = HashSet::new();

        for ip in net.ip{
            for (option_id, value) in ip.options {
                match option_id.to_ascii_lowercase().as_str(){
                    "range" => {
                        let temp : Vec<&str> = value.split("-").collect();
                        // TODO: This may need to be a real error
                        assert_eq!(temp.len(), 2);
                        
                        let beg_range = temp[0].replace(".", "").parse::<u32>().expect("Invalid beginning IP range number");
                        let end_range = temp[1].replace(".", "").parse::<u32>().expect("Invalid ending IP range number");

                        if end_range < beg_range {
                            // TODO: error
                        }

                        for i in beg_range..end_range+1 {
                            if ip_list.contains(&i){
                                // TODO: error
                            }

                            ip_list.insert(i);

                            ips.push((i.into(), network));
                        }

                    },
                    "ip" => {
                        // Ipv4Address::new([123, 45, 67, 89]);
                        // let temp : Vec<&str> = value.split(".").collect();
                        // let mut new_ip : Vec<u8> = Vec::new();
                        // for val in temp {
                        //     new_ip.push(val.parse::<u8>().expect("Invalid IP octet"));
                        // }

                        // if new_ip.len() < 4 {
                        //     // TODO: error
                        // }

                        // let real_ip = [new_ip[0], new_ip[1], new_ip[2], new_ip[3]];
                        let new_ip = value.replace(".", "").parse::<u32>().expect("Invalid beginning IP range number");

                        if ip_list.contains(&new_ip){
                            // TODO: error
                        }
                        
                        ip_list.insert(new_ip);

                        ips.push((new_ip.into(), network));
                    }
                    _ => {
                        // TODO: error case
                    }
                }
            }
        }
        let ip_table: IpToNetwork = ips.into_iter().collect();
        networks.insert(id, (network, ip_table));
    }
    Ok(networks)
}