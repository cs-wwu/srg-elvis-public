//! Generates machines from a given parse
use std::collections::HashMap;

use crate::ndl::generating::{application_generator::*, protocol_generator::*, generator_utils::ip_string_to_ip};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::machine::ProtocolMapBuilder;
use elvis_core::protocols::ipv4::{Ipv4Address, Recipient};
use elvis_core::protocols::Pci;
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
            // Create a name to ip mapping
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
                        

                        let ip = ip_string_to_ip(if app_name == "send_message" {
                            app.options.get("ip")
                                .map_or("127.0.0.1".to_string(), |ip_str| ip_str.to_string())
                        } else {
                            app.options.get("ip")
                                .unwrap_or_else(|| panic!("{app_name} application doesn't contain ip."))
                                .to_string()
                        }, "Application IP");

                        // TODO uncomment when ndl can support multiple local
                        // ips via unique names (can be done now but will require changing lots of .ndl files)

                        // Create a unique name for machines with multiple local ips
                        // let new_mach_name = match app_name {
                        //     "capture" => cur_name.clone() + "-capture",
                        //     "forward" => cur_name.clone() + "-forward",
                        //     "ping_pong" => cur_name.clone() + "-ping-pong",
                        //     "send_message" => cur_name.clone() + "-send-message",
                        //     _ => panic!("Unsupported application encountered: {:?}", app_name),
                        // };
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
            let mut networks_to_be_added = Vec::new();
            let mut protocol_map = ProtocolMapBuilder::new();
            let mut ip_table = IpTable::<Recipient>::new();

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
                networks_to_be_added.push(network_adding.clone());
            }
            for app in &machine.interfaces.applications {
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                match app_name {
                    "send_message" => {
                        protocol_map = protocol_map.with(send_message_builder(app, &name_to_ip, &mut ip_table, &mut ip_gen))
                    }

                    "capture" => {
                        protocol_map = protocol_map.with(capture_builder(app, &mut ip_table, &mut ip_gen));
                    }

                    "forward" => {
                        protocol_map = protocol_map.with(forward_message_builder(app, &name_to_ip, &mut ip_table, &mut ip_gen))
                    }

                    "ping_pong" => {
                        protocol_map = protocol_map.with(ping_pong_builder(
                            app,
                            &name_to_ip,
                            &mut ip_table,
                            &mut ip_gen
                        ))
                    }

                    _ => {
                        panic!("Invalid application in machine. Got application {app_name}");
                    }
                }
            }
            protocol_map = protocol_map.with(Pci::new(networks_to_be_added));
            for protocol in &machine.interfaces.protocols {
                for option in &protocol.options {
                    match option.1.as_str() {
                        "UDP" => protocol_map = protocol_map.with(Udp::new()),
                        "IPv4" => protocol_map = protocol_map.with(Ipv4::new(ip_table.clone())),
                        "ARP" => protocol_map = protocol_map.with(arp_builder()),
                        _ => {
                            panic!(
                                "Invalid Protocol found in machine. Found: {}",
                                option.1.as_str()
                            )
                        }
                    }
                }
            }
            machine_list.push(elvis_core::Machine::new(protocol_map.build()));
        }
    }
    machine_list
}
