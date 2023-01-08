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
// Note, the same IP between two different networks seems to break the sim

pub async fn core_generator(s: Sim) {
    println!("{:?}", s);
    let mut internet = Internet::new();
    // println!("{:?}", network_generator(s.networks, internet));
    let networks = network_generator(s.networks, &mut internet).unwrap();
    let machines = machine_generator(s.machines, &mut internet, networks);

    internet.run().await;
}

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


fn machine_generator(m: Machines, internet: &mut Internet, networks: HashMap<String, (NetworkHandle, IpToNetwork, HashSet<[u8; 4]>)>){
    // println!("Printing machines: {:?}", m);
    // For now options are ignored as we can't currently add names/IDs to machines
    // Focusing on Interfaces, protocols, and applications
    for machine in &m {
        // println!("\n interfaces: {:?}", machine.interfaces);
        let mut networks_to_be_added = Vec::new();
        let mut protocols_to_be_added = Vec::new();
        let mut ip_table: IpToNetwork = IpToNetwork::new();
        for net in &machine.interfaces.networks{
            // TODO: test and change errors
            assert!(networks.contains_key(net.options.get("id").expect("Invalid ID found")), "Invalid ID found assert");
            let network_adding = networks.get(net.options.get("id").unwrap()).unwrap();
            networks_to_be_added.push(network_adding.0);
            
            // add the IP table found into our existing IP table
            ip_table.extend(network_adding.1.clone());
        }

        for protocol in &machine.interfaces.protocols{
            // println!("\nProtocol: {:?}", protocol);
            for option in &protocol.options{
                match option.1.as_str(){
                    "UDP" => {
                        protocols_to_be_added.push(Udp::new_shared() as SharedProtocol)
                    },
                    "IPv4" => {
                        protocols_to_be_added.push(Ipv4::new_shared(ip_table.clone()))
                    },
                    _ =>{
                        // TODO: when machine ID/name get found, add to the error
                        panic!("Invalid Protocol found in machine")
                    }
                }
            } 
        }
        for app in &machine.interfaces.applications{
            println!("\nApplication: {:?}", app);
            // TODO: assert to check for name
            let app_name = app.options.get("name").unwrap().as_str();
            match app_name{
                "send_message" => {
                    // TODO: write the error messages for these asserts
                    assert!(app.options.contains_key("port"));
                    assert!(app.options.contains_key("to"));
                    assert!(app.options.contains_key("message"));
                    
                    // TODO: edit this error message?
                    let to = ip_string_to_ip(app.options.get("to").unwrap().to_string(), "send_message declaration");
                    let port = string_to_port(app.options.get("port").unwrap().to_string());  
                    let message = app.options.get("message").unwrap().to_owned();
                    
                    //TODO: ask Tim about this message Box stuff
                    protocols_to_be_added.push(SendMessage::new_shared(Box::leak(message.into_boxed_str()), to.into(), port));
                },

                "capture" => {
                    assert!(app.options.contains_key("port"));
                    assert!(app.options.contains_key("ip"));
                    let ip = ip_string_to_ip(app.options.get("ip").unwrap().to_string(), "capture declaration");
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    protocols_to_be_added.push(Capture::new_shared(ip.into(), port));
                },

                _ => {
                    
                }
            }
        }
        
        internet.machine(
            protocols_to_be_added, 
            networks_to_be_added
        );
    }
}

fn string_to_port(p: String) -> u16{
    if p.starts_with("0x"){
        return u16::from_str_radix(&p[2..], 16).expect(&format!("Port declaration error. Found port: {}", p));
    }
    else {
        return p.parse::<u16>().expect(&format!("Invalid number for port. Found port: {}", p));
    }
}

// internet.machine(
//     [
//         Udp::new_shared() as SharedProtocol,
//         Ipv4::new_shared(networks.get("1").unwrap().1.clone()),
//     ],
//     networks_to_be_added,
// );
