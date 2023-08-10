//! Generates machines from a given parse
use std::collections::{HashMap, HashSet};

use crate::ndl::generating::{
    application_generator::*, generator_utils::ip_string_to_ip,
};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::machine::ProtocolMapBuilder;
use elvis_core::protocols::ipv4::{Ipv4Address, Recipient};
use elvis_core::protocols::Pci;
use elvis_core::protocols::Arp;
use elvis_core::protocols::{ipv4::Ipv4, udp::Udp};
use elvis_core::IpTable;
use itertools::Itertools;

use super::generator_data::NetworkInfo;

/// Machine Generator generates machines from a given [Machines] struct and places them in the [Internet]
pub fn machine_generator(machines: Machines, networks: &NetworkInfo) -> Vec<elvis_core::Machine> {

    // Focusing on Interfaces, protocols, and applications
    let mut name_to_ip: HashMap<String, Ipv4Address> = HashMap::new();
    let mut ip_gen = networks.ip_hash.clone();

    for machine in machines.iter() {
        let mut cur_name: String = String::new();
        let mut machine_count: u64 = 1;

        // Get the machine count if there is one
        if machine.options.is_some() && machine.options.as_ref().unwrap().contains_key("count") {
            machine_count = machine
                .options
                .as_ref()
                .unwrap()
                .get("count")
                .unwrap()
                .parse::<u64>()
                .unwrap_or_else(|_| {
                    panic!(
                        "Invalid count argument in machine. Expected u64 and found: {}",
                        machine.options.as_ref().unwrap().get("count").unwrap()
                    )
                });
            assert!(machine_count > 0, "Machine count less than 1.");
        }
        // Loop through the count for each machine
        for temp_machine_count in 0..machine_count {

            // Create a name for each machine where one is specified
            // If the machine > 1 append the number to maintain unique names
            if machine.options.is_some() && machine.options.as_ref().unwrap().contains_key("name") {
                cur_name = machine
                    .options
                    .as_ref()
                    .unwrap()
                    .get("name")
                    .unwrap()
                    .to_string();
                if machine_count > 1 {
                    cur_name = cur_name + "-" + &temp_machine_count.to_string();
                }
            }
            // Create a name to ip mapping if a name has been provided
            if !cur_name.is_empty() {
                for app in &machine.interfaces.applications {
                    assert!(
                        app.options.contains_key("name"),
                        "Machine application does not contain a name"
                    );
                    let app_name = app.options.get("name").unwrap().as_str();
                    if app_name == "capture" || app_name == "forward" || app_name == "ping_pong" || app_name == "send_message" {
                        assert!(
                            app.options.contains_key("ip") || app_name == "send_message",
                            "{app_name} application doesn't contain ip."
                        );

                        // This check makes sure counts do not appear on recieving machines.
                        // Can be removed when ELVIS allows for this.
                        assert!(
                            machine_count == 1  || app_name == "send_message",
                            "Machine {cur_name} contains count and {app_name} application"
                        );
                        
                        // Get the local ip of the application
                        let ip = ip_string_to_ip(if app_name == "send_message" {
                            app.options.get("ip")
                                .map_or("127.0.0.1".to_string(), |ip_str| ip_str.to_string())
                        } else {
                            app.options.get("ip")
                                .unwrap_or_else(|| panic!("{app_name} application doesn't contain ip."))
                                .to_string()
                        }, "Application IP");

                        name_to_ip.insert(cur_name.clone(), ip.into());
                    }
                }
            }
        }
    }
    let mut machine_list = Vec::new();
    for machine in &machines {
        let mut machine_count = 1;
        let mut _cur_machine_name: String;
        if machine.options.is_some() {

            // Parse machine parameters if there are any
            for option in machine.options.as_ref().unwrap() {
                match option.0.as_str() {
                    // TODO: Checks may be able to be removed as we checked up above in the stack
                    "count" => {
                        machine_count = option.1.parse::<u64>().unwrap_or_else(|_| {
                            panic!(
                                "Invalid count argument in machine. Expected u64 and found: {}",
                                option.1
                            )
                        });
                        assert!(machine_count > 0, "Machine count less than 1.");
                    }
                    "name" => {
                        _cur_machine_name = option.1.clone();
                    }
                    
                    _ => {}
                }
            }
        }

        for _count in 0..machine_count {
            let mut net_ids = Vec::new();
            let mut networks_to_be_added = Vec::new();
            let mut protocol_map = ProtocolMapBuilder::new();
            let mut ip_table = IpTable::<Recipient>::new();

            //add networks to the machine
            for net in machine.interfaces.networks.iter() {
                // TODO: maybe still need an error test
                assert!(
                    networks.nets.contains_key(
                        net.options
                            .get("id")
                            .expect("No ID found in network being added to machine.")
                    ),
                    "Invalid Network ID found. Got {} expected {:?}",
                    net.options.get("id").unwrap(),
                    networks.nets.keys().sorted().join(" , ")
                );
                //Save the relevant network id's and their corresponding data for later use
                let net_id = net.options.get("id").unwrap();
                let network_adding = networks.nets.get(net_id).unwrap();
                net_ids.push(net_id.to_string());
                networks_to_be_added.push(network_adding.clone());
            }
            protocol_map = protocol_map.with(Pci::new(networks_to_be_added));
            
            //build all apps the machine has
            for app in &machine.interfaces.applications {
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                match app_name {
                    "send_message" => {
                        protocol_map = protocol_map.with(send_message_builder(app, &name_to_ip, &mut ip_table, &mut ip_gen, &net_ids))
                    }

                    "capture" => {
                        protocol_map = protocol_map.with(capture_builder(app, &mut ip_table, &mut ip_gen, &net_ids));
                    }

                    "forward" => {
                        protocol_map = protocol_map.with(forward_message_builder(app, &name_to_ip, &mut ip_table, &mut ip_gen, &net_ids))
                    }

                    "ping_pong" => {
                        protocol_map = protocol_map.with(ping_pong_builder(
                            app,
                            &name_to_ip,
                            &mut ip_table,
                            &mut ip_gen,
                            &net_ids
                        ))
                    }

                    _ => {
                        panic!("Invalid application in machine. Got application {app_name}");
                    }
                }
            }

            // List of required protocols for each machine. Since they are required
            // a default version of the protocal can be automatically added to the machines
            // without needing user input in NDL files. If the user prefers to specify
            // them that is also supported.
            let required_protocols: HashSet<&str> = ["IPv4", "ARP"].iter().copied().collect();
            let mut encountered_protocols: HashSet<&str> = HashSet::new();

            // Creates the user specified protocols and adds them to the protocol map
            // Generated protocols are recorded to allow automatic addition of required protocols
            for protocol in &machine.interfaces.protocols {
                for option in &protocol.options {
                    match option.1.as_str() {
                        "UDP" => protocol_map = protocol_map.with(Udp::new()),
                        "IPv4" => protocol_map = protocol_map.with(Ipv4::new(ip_table.clone())),
                        "ARP" => protocol_map = protocol_map.with(arp_builder(
                            &name_to_ip,
                            &protocol.options)),
                        _ => {
                            panic!(
                                "Invalid Protocol found in machine. Found: {}",
                                option.1.as_str()
                            )
                        }
                    }
                    // Add the encountered protocol name to the HashSet
                    encountered_protocols.insert(option.1.as_str());
                }
            }

            // Check for missing required protocols and add them if necessary
            for required_protocol in &required_protocols {
                if !encountered_protocols.contains(required_protocol) {
                    match *required_protocol {
                        "IPv4" => protocol_map = protocol_map.with(Ipv4::new(ip_table.clone())),
                        "ARP" => protocol_map = protocol_map.with(Arp::basic()),
                        _ => {
                            panic!("Missing required protocol: {}", required_protocol);
                        }
                    }
                }
            }

            machine_list.push(elvis_core::Machine::new(protocol_map.build()));
        }
    }
    machine_list
}
