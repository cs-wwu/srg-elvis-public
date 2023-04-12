//! Generates machines from a given parse
use std::collections::HashMap;

use crate::ndl::generating::{application_generator::*, generator_utils::ip_string_to_ip};
use crate::ndl::parsing::parsing_data::*;
use elvis_core::network::Mac;
use elvis_core::protocols::ipv4::{Ipv4Address, Recipient};
use elvis_core::protocols::Pci;
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp},
};
use itertools::Itertools;
use rustc_hash::FxHashMap;

use super::generator_data::NetworkInfo;

/// Machine Generator generates machines from a given [Machines] struct and places them in the [Internet]
pub fn machine_generator(machines: Machines, networks: &NetworkInfo) -> Vec<elvis_core::Machine> {
    // Focusing on Interfaces, protocols, and applications
    let mut name_to_mac: HashMap<String, Mac> = HashMap::new();
    let mut name_to_ip: HashMap<String, Ipv4Address> = HashMap::new();
    let mut ip_to_mac: HashMap<Ipv4Address, Mac> = HashMap::new();
    let mut cur_mac: u64 = 0;
    for machine in machines.iter() {
        let mut cur_name: String = String::new();
        if machine.options.is_some() && machine.options.as_ref().unwrap().contains_key("count") {
            let machine_count = machine
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
            cur_mac += machine_count - 1;
        }

        if machine.options.is_some() && machine.options.as_ref().unwrap().contains_key("name") {
            cur_name = machine
                .options
                .as_ref()
                .unwrap()
                .get("name")
                .unwrap()
                .to_string();
            name_to_mac.insert(cur_name.clone(), cur_mac);
        }

        if !cur_name.is_empty() {
            for app in &machine.interfaces.applications {
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                if app_name == "capture" || app_name == "forward" || app_name == "ping_pong" {
                    assert!(
                        app.options.contains_key("ip"),
                        "{app_name} application doesn't contain ip."
                    );

                    let ip = ip_string_to_ip(
                        app.options.get("ip").unwrap().to_string(),
                        format!("{app_name} declaration").as_str(),
                    );

                    name_to_ip.insert(cur_name.clone(), ip.into());
                    ip_to_mac.insert(ip.into(), cur_mac);
                }
            }
        }

        cur_mac += 1;
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
            let mut protocols_to_be_added = Vec::new();
            let mut ip_table = FxHashMap::default();

            for (net_num, net) in (0_u32..).zip(machine.interfaces.networks.iter()) {
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
                let network_adding = networks.nets.get(net.options.get("id").unwrap()).unwrap();
                networks_to_be_added.push(network_adding.clone());

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
                    let mac = ip_to_mac.get(ip).cloned();
                    ip_table.insert(*ip, Recipient::new(net_num, mac));
                }
            }
            protocols_to_be_added.push(Pci::new(networks_to_be_added).shared());
            for protocol in &machine.interfaces.protocols {
                for option in &protocol.options {
                    match option.1.as_str() {
                        "UDP" => protocols_to_be_added.push(Udp::new().shared() as SharedProtocol),
                        "IPv4" => protocols_to_be_added.push(Ipv4::new(ip_table.clone()).shared()),
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
                assert!(
                    app.options.contains_key("name"),
                    "Machine application does not contain a name"
                );
                let app_name = app.options.get("name").unwrap().as_str();
                match app_name {
                    "send_message" => {
                        protocols_to_be_added.push(send_message_builder(app, &name_to_ip))
                    }

                    "capture" => {
                        protocols_to_be_added.push(capture_builder(app));
                    }

                    "forward" => {
                        protocols_to_be_added.push(forward_message_builder(app, &name_to_ip))
                    }

                    "ping_pong" => protocols_to_be_added.push(ping_pong_builder(
                        app,
                        &name_to_ip,
                        &ip_to_mac,
                        &name_to_mac,
                    )),

                    _ => {
                        panic!("Invalid application in machine. Got application {app_name}");
                    }
                }
            }

            machine_list.push(elvis_core::Machine::new(protocols_to_be_added));
        }
    }
    machine_list
}
