//! Generates machines from a given parse
use std::collections::HashMap;

use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::network::Mac;
use elvis_core::protocols::ipv4::{IpToTapSlot, Ipv4Address};
use elvis_core::protocols::Pci;
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp},
};
use itertools::Itertools;

use super::generator_data::NetworkInfo;

/// Machine Generator generates machines from a given [Machines] struct and places them in the [Internet]
pub fn machine_generator(m: Machines, networks: &NetworkInfo) -> Vec<elvis_core::Machine> {
    // Focusing on Interfaces, protocols, and applications
    let mut name_to_mac: HashMap<String, Mac> = HashMap::new();
    let mut name_to_ip: HashMap<String, Ipv4Address> = HashMap::new();
    let mut cur_mac: Mac = 0;

    for machine in &m {
        let mut cur_name: String = String::new();
        if machine.options.is_some() {
            if machine.options.as_ref().unwrap().contains_key("name"){
                cur_name = machine.options.as_ref().unwrap().get("name").unwrap().to_string();
                name_to_mac.insert(cur_name.clone(), cur_mac);
            }
        }
        
        if cur_name != "" {
            for app in &machine.interfaces.applications {
                // TODO: add test for this error
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                if app_name == "capture" {
                    assert!(
                        app.options.contains_key("ip"),
                        "Capture application doesn't contain ip."
                    );

                    let ip = ip_string_to_ip(
                        app.options.get("ip").unwrap().to_string(),
                        "capture declaration",
                    );

                    name_to_ip.insert(cur_name.clone(), ip.into());
                    
                }
            }
        }
        cur_mac += 1;
    }
    println!("{:?} and then ips {:?}", name_to_mac, name_to_ip);

    let mut machine_list = Vec::new();

    for machine in &m {
        let mut machine_count = 1;
        // let mut machine_name: String;
        if machine.options.is_some() {
            for option in machine.options.as_ref().unwrap() {
                match option.0.as_str() {
                    "count" => {
                        machine_count = option.1.parse::<u32>().unwrap_or_else(|_| {
                            panic!(
                                "Invalid count argument in machine. Expected u32 and found: {}",
                                option.1
                            )
                        });
                        assert!(machine_count > 0, "Machine count less than 1.");
                    }
                    "name" => {
                        // Set machine name once possible
                        // machine_name = option.1.to_owned();
                    }
                    _ => {}
                }
            }
        }

        // println!("count is: {}", machine_count);

        for _count in 0..machine_count {
            let mut networks_to_be_added = Vec::new();
            let mut protocols_to_be_added = Vec::new();
            let mut ip_table = Vec::new();

            for (net_num, net) in (0_u32..).zip(machine.interfaces.networks.iter()) {
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
                let network_adding = networks.nets.get(net.options.get("id").unwrap()).unwrap();
                networks_to_be_added.push(network_adding.tap());

                // TODO: Add test for this expect?
                let ips = networks
                    .ip_hash
                    .get(net.options.get("id").unwrap())
                    .unwrap_or_else(|| {
                        panic!(
                            "No IPs found for network with id {}",
                            net.options.get("id").unwrap()
                        )
                    });
                for ip in ips {
                    ip_table.push((*ip, net_num));
                }
            }
            let ip_table: IpToTapSlot = ip_table.into_iter().collect();
            protocols_to_be_added.push(Pci::new_shared(networks_to_be_added));
            for protocol in &machine.interfaces.protocols {
                for option in &protocol.options {
                    match option.1.as_str() {
                        "UDP" => protocols_to_be_added.push(Udp::new_shared() as SharedProtocol),
                        "IPv4" => protocols_to_be_added.push(Ipv4::new_shared(ip_table.clone())),
                        _ => {
                            panic!(
                                "Invalid Protocol found in machine. Found: {}",
                                option.1.as_str()
                            )
                        }
                    }
                }
            }
            for app in &machine.interfaces.applications {
                // TODO: add test for this error
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                match app_name {
                    "send_message" => {
                        assert!(
                            app.options.contains_key("port"),
                            "Send_Message application doesn't contain port."
                        );
                        assert!(
                            app.options.contains_key("to"),
                            "Send_Message application doesn't contain to address."
                        );
                        assert!(
                            app.options.contains_key("message"),
                            "Send_Message application doesn't contain message."
                        );

                        // let to = ip_string_to_ip(
                        //     app.options.get("to").unwrap().to_string(),
                        //     "send_message declaration",
                        // );
                        // TODO: might want to include both these behaviors (so they could enter IP or enter name)
                        let to = app.options.get("to").unwrap().to_string();
                        let port = string_to_port(app.options.get("port").unwrap().to_string());
                        let message = app.options.get("message").unwrap().to_owned();

                        //TODO: ask Tim about this message Box stuff
                        // TODO: Add count and MAC to parser
                        // TODO: add error test for invalid name
                        protocols_to_be_added.push(SendMessage::new_shared(
                            Box::leak(message.into_boxed_str()),
                            *name_to_ip.get(&to).expect("Invalid name for 'to' in send_message"),
                            port,
                            // TODO: This should be a var not static set to first machine
                            Some(*name_to_mac.get(&to).expect("Invalid name for 'to' in send_message")),
                            1,
                        ));
                    }

                    "capture" => {
                        assert!(
                            app.options.contains_key("port"),
                            "Capture application doesn't contain port."
                        );
                        assert!(
                            app.options.contains_key("ip"),
                            "Capture application doesn't contain ip."
                        );
                        // TODO: Add error check
                        assert!(
                            app.options.contains_key("message_count"),
                            "Capture application doesn't contain message_count."
                        );

                        // TODO: Check that this IP is valid in the IP table/Network
                        let ip = ip_string_to_ip(
                            app.options.get("ip").unwrap().to_string(),
                            "capture declaration",
                        );
                        let port = string_to_port(app.options.get("port").unwrap().to_string());
                        let message_count = app.options.get("message_count").unwrap().parse::<u32>().expect("Invalid u32 found in Capture for message count");
                        // TODO: Figure out how to get actual number to recieve in
                        // TODO: Add message expected count
                        // maybe default to 1?
                        protocols_to_be_added.push(Capture::new_shared(ip.into(), port, message_count));
                    }

                    _ => {
                        panic!(
                            "Invalid application in machine. Got application {}",
                            app_name
                        );
                    }
                }
            }

            machine_list.push(elvis_core::Machine::new(protocols_to_be_added));
        }
    }

    machine_list
}
