//! Generates machines from a given parse
use crate::ndl::parsing::parsing_data::*;
use crate::{
    applications::{Capture, SendMessage},
    ndl::generating::generator_utils::{ip_string_to_ip, string_to_port},
};
use elvis_core::protocols::ipv4::IpToTapSlot;
use elvis_core::protocols::Pci;
use elvis_core::{
    protocol::SharedProtocol,
    protocols::{ipv4::Ipv4, udp::Udp},
};
use itertools::Itertools;

use super::generator_data::NetworkInfo;

/// Machine Generator generates machines from a given [Machines] struct and places them in the [Internet]
pub fn machine_generator(m: Machines, networks: &NetworkInfo) -> Vec<elvis_core::Machine> {
    // For now options are ignored as we can't currently add names/IDs to machines
    // Focusing on Interfaces, protocols, and applications
    let mut machine_list = Vec::new();
    for machine in &m {
        let mut networks_to_be_added = Vec::new();
        let mut protocols_to_be_added = Vec::new();
        let mut ip_table = Vec::new();
        let mut net_num: u32 = 0;
        for net in &machine.interfaces.networks {
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
                .expect(&format!(
                    "No IPs found for network with id {}",
                    net.options.get("id").unwrap()
                ));
            for ip in ips {
                ip_table.push((*ip, net_num));
            }
            net_num += 1;
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

                    let to = ip_string_to_ip(
                        app.options.get("to").unwrap().to_string(),
                        "send_message declaration",
                    );
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    let message = app.options.get("message").unwrap().to_owned();

                    //TODO: ask Tim about this message Box stuff
                    // TODO: Add count and MAC to parser
                    protocols_to_be_added.push(SendMessage::new_shared(
                        Box::leak(message.into_boxed_str()),
                        to.into(),
                        port,
                        None,
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

                    // TODO: Check that this IP is valid in the IP table/Network
                    let ip = ip_string_to_ip(
                        app.options.get("ip").unwrap().to_string(),
                        "capture declaration",
                    );
                    let port = string_to_port(app.options.get("port").unwrap().to_string());
                    protocols_to_be_added.push(Capture::new_shared(ip.into(), port));
                }

                _ => {
                    panic!(
                        "Invalid application in machine. Got application {}",
                        app_name
                    );
                }
            }
        }
        // Add to the protocol list:
        //  Pci::new_shared([network.tap()]),
        machine_list.push(elvis_core::Machine::new(protocols_to_be_added));
    }

    return machine_list;
}
