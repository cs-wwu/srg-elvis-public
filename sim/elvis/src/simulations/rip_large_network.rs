use crate::applications::{
    arp_router::RoutingTable, rip::rip_router::RipRouter, ArpRouter, Counter, MultiCapture,
    SendMessage,
};
use elvis_core::{
    new_machine,
    protocols::{
        arp::subnetting::{Ipv4Mask, SubnetInfo, Ipv4Net},
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Arp, Endpoint, Pci,
    },
    run_internet_with_timeout,
    shutdown::ExitStatus,
    IpTable, Machine, Message, Network,
};
use std::{sync::Arc, time::Duration};

// Routes a message from a SENDER machine to all CAP (capture) machines
// ----------------------------------------------------------------------
// KEY
// Router: R(x)
// Machine: M(x)
// Network: N(x)
// where x is an identification
// ----------------------------------------------------------------------
// Networks:
// N1 = 10.0.0.0/30
// N2 = 10.0.0.8/29
// N3 = 10.0.0.4/30
// N4 = 10.0.0.16/30
// N5 = 10.0.0.20/30
// N6 = 10.0.0.24/29
// N7 = 10.0.0.32/30
// ----------------------------------------------------------------------
// M(SENDER) -- N(N1) -- R(RIP1) -- N(N2) -- R(RIP2) -- N(N3) -- M(CAP1)
//                                  |
//                              R(RIP3)
//                            /        \
//                          N(N4)      N(N5)
//                         /             \
//                     R(RIP4)          R(RIP5)
//                      /                  \
//        M(CAP2) -- N(N6) -- M(CAP3)      N(N7) -- M(CAP4)

const MESSAGE_PORT: u16 = 0xdeeb;

// I would ideally use the static variable HOST_ADDRESSES here but we would need to unwrap Ipv4Net which is not a const operation
fn get_hosts() -> [Ipv4Net; 5] {
    [
        // SENDER
        Ipv4Net::from_cidr("10.0.0.1/30").expect("Address is properly formatted"),
        // CAPTURE 1
        Ipv4Net::from_cidr("10.0.0.6/30").expect("Address is properly formatted"),
        // CAPTURE 2
        Ipv4Net::from_cidr("10.0.0.26/29").expect("Address is properly formatted"),
        // CAPTURE 3
        Ipv4Net::from_cidr("10.0.0.27/29").expect("Address is properly formatted"),
        // CAPTURE 4
        Ipv4Net::from_cidr("10.0.0.34/30").expect("Address is properly formatted"),
    ]
}
fn router_1_interfaces() -> [Ipv4Net; 2] {
    [
        // Interface to N1
        Ipv4Net::from_cidr("10.0.0.2/30").expect("Address is properly formatted"),
        // Interface to N2
        Ipv4Net::from_cidr("10.0.0.9/29").expect("Address is properly formatted"),
    ]
}

fn router_2_interfaces() -> [Ipv4Net; 2] {
    [
        // Interface to N2
        Ipv4Net::from_cidr("10.0.0.10/29").expect("Address is properly formatted"),
        // Interface to N3
        Ipv4Net::from_cidr("10.0.0.5/30").expect("Address is properly formatted"),
    ]
}

fn router_3_interfaces() -> [Ipv4Net; 3] {
    [
        // Interface to N2
        Ipv4Net::from_cidr("10.0.0.11/29").expect("Address is properly formatted"),
        // Interface to N4
        Ipv4Net::from_cidr("10.0.0.17/30").expect("Address is properly formatted"),
        // Interface to N5
        Ipv4Net::from_cidr("10.0.0.21/30").expect("Address is properly formatted"),
    ]
}

fn router_4_interfaces() -> [Ipv4Net; 2] {
    [
        // Interface to N4
        Ipv4Net::from_cidr("10.0.0.18/30").expect("Address is properly formatted"),
        // Interface to N6
        Ipv4Net::from_cidr("10.0.0.25/29").expect("Address is properly formatted"),
    ]
}

fn router_5_interfaces() -> [Ipv4Net; 2] {
    [
        // Interface to N5
        Ipv4Net::from_cidr("10.0.0.22/30").expect("Address is properly formatted"),
        // Interface to N7
        Ipv4Net::from_cidr("10.0.0.33/30").expect("Address is properly formatted"),
    ]
}

pub fn create_capture(
    subnet: Ipv4Net,
    default_gateway: Ipv4Address,
    network: Arc<Network>,
    multicapture_counter: Arc<Counter>,
) -> Machine {
    new_machine![
        Pci::new([network]),
        Arp::new().preconfig_subnet(subnet.addr(), SubnetInfo::new(subnet.mask(), default_gateway)),
        Ipv4::new(Default::default()),
        Udp::new(),
        MultiCapture::new(Endpoint::new(subnet.addr(), MESSAGE_PORT), multicapture_counter)
    ]
}

pub fn create_router(
    // Attached networks
    networks: impl IntoIterator<Item = Arc<Network>>,
    // Router's interface IPs
    interface_ips: &[Ipv4Net],
    // Routing table to end devices
    routing_table: RoutingTable,
) -> Machine {
    // IPs are mapped to interfaces/pcis (of networks) based on their order
    // E.g. the first address in interface_ips will be the ip of the first pci interface

    let mut interfaces = IpTable::<Recipient>::new();
    for (pci_slot, addr) in interface_ips.iter().enumerate() {
        interfaces.add(*addr, Recipient::new(pci_slot as u32, None));
    }

    new_machine![
        Pci::new(networks),
        Arp::new(),
        Ipv4::new(interfaces),
        Udp::new(),
        ArpRouter::new(),
        RipRouter::new(),
    ]
}

#[allow(non_snake_case)]
pub async fn rip_large_network(capture_ips: Vec<Ipv4Net>) -> ExitStatus {
    // All these variables should be defined as const 
    // but because we need to extract the result (a non const operation)
    // we are forced to do use non-const functions
    let HOST_ADDRESSES = get_hosts();
    let ROUTER_1_INTERFACES = router_1_interfaces();
    let ROUTER_2_INTERFACES = router_2_interfaces();
    let ROUTER_3_INTERFACES = router_3_interfaces();
    let ROUTER_4_INTERFACES = router_4_interfaces();
    let ROUTER_5_INTERFACES = router_5_interfaces();

    // Create 7 basic networks
    // Network::basic() :   mtu = maximum packet size;
    //                      throughput = amount of data successfully transmitted from x to y in a fixed amount of time
    //                      latency = simulated packet transit time
    let networks: Vec<Arc<Network>> = (0..7).map(|_| Network::basic()).collect();

    // Create a lists of endpoints for capture machines
    let mut endpoints = Vec::new();
    capture_ips
        .iter()
        .for_each(|recipient_ip| endpoints.push(Endpoint::new(recipient_ip.addr(), MESSAGE_PORT)));

    // Number of recipients = numebr of capture_ips
    let multicapture_counter = Counter::new(capture_ips.len() as u32);

    // Only sending message to CAP3
    let message = SendMessage::with_endpoints(vec![Message::new(b"Yahoo")], endpoints)
        .delay(Duration::from_secs(3));

    // Everything is a machine
    let mut end_devices = vec![
        // SENDER MACHINE
        new_machine![
            // Pci attached to network 1
            Pci::new([networks[0].clone()]),
            // Host IP configuration
            Arp::new().preconfig_subnet(
                // Sender IP
                HOST_ADDRESSES[0].addr(),
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(30),
                    default_gateway: ROUTER_1_INTERFACES[0].addr()
                }
            ),
            // IPv4 protocol intended to send message from ip HOST_ADDRESSES[0] out Pci slot 0
            Ipv4::new(IpTable::from_iter(
                [(HOST_ADDRESSES[0], Recipient::new(0, None))].into_iter()
            )),
            // Using transport protocol: udp
            Udp::new(),
            message.local_ip(HOST_ADDRESSES[0].addr()),
            MultiCapture::new(
                Endpoint::new(HOST_ADDRESSES[0].addr(), MESSAGE_PORT),
                multicapture_counter.clone()
            )
        ],
    ];

    let captures = vec![
        // Capture 1
        create_capture(
            // Address of machine
            HOST_ADDRESSES[1],
            ROUTER_2_INTERFACES[1].addr(),
            // Attached network
            networks[2].clone(),
            // Multicapture counter and status
            multicapture_counter.clone(),
        ),
        // Capture 2
        create_capture(
            HOST_ADDRESSES[2],
            ROUTER_4_INTERFACES[1].addr(),
            networks[5].clone(),
            multicapture_counter.clone(),
        ),
        // Capture 3
        create_capture(
            HOST_ADDRESSES[3],
            ROUTER_4_INTERFACES[1].addr(),
            networks[5].clone(),
            multicapture_counter.clone(),
        ),
        // Capture 4
        create_capture(
            HOST_ADDRESSES[4],
            ROUTER_5_INTERFACES[1].addr(),
            networks[6].clone(),
            multicapture_counter.clone(),
        ),
    ];
    end_devices.extend(captures);

    let mut routers = vec![
        // RIP 1
        create_router(
            // Connected networks
            [networks[0].clone(), networks[1].clone()],
            &ROUTER_1_INTERFACES,
            // Connected hosts
            [(HOST_ADDRESSES[0], (None, 1))].into_iter().collect(),
        ),
        // RIP 2
        create_router(
            [networks[1].clone(), networks[2].clone()],
            &ROUTER_2_INTERFACES,
            [(HOST_ADDRESSES[1], (None, 1))].into_iter().collect(),
        ),
        // RIP 3
        create_router(
            [
                networks[1].clone(),
                networks[3].clone(),
                networks[4].clone(),
            ],
            &ROUTER_3_INTERFACES,
            // RIP router is connected to no hosts
            RoutingTable::new(),
        ),
        // RIP 4
        create_router(
            [networks[3].clone(), networks[5].clone()],
            &ROUTER_4_INTERFACES,
            [
                (HOST_ADDRESSES[2], (None, 1)),
                (HOST_ADDRESSES[3], (None, 1)),
            ]
            .into_iter()
            .collect(),
        ),
        // RIP 5
        create_router(
            [networks[4].clone(), networks[6].clone()],
            &ROUTER_5_INTERFACES,
            [(HOST_ADDRESSES[4], (None, 1))].into_iter().collect(),
        ),
    ];

    routers.extend(end_devices);
    let machines = routers;

    run_internet_with_timeout(&machines, Duration::from_secs(10)).await
}

#[cfg(test)]
mod tests {

    use super::*;

    #[allow(non_snake_case)]
    #[tokio::test]
    async fn rip_large_network() {
        // SINGLE CAPTURE (SENDER -> CAPTURE3)
        let HOST_ADDRESSES = get_hosts();
        let recipient_ips = Vec::from([HOST_ADDRESSES[3]]);
        let test1 = super::rip_large_network(recipient_ips.clone());

        // Message should reach capture 3 (and no other)
        assert_eq!(
            test1.await,
            super::ExitStatus::Status(recipient_ips.len() as u32)
        );
    }

    #[allow(non_snake_case)]
    #[tokio::test]
    async fn rip_large_network_all() {
        let HOST_ADDRESSES = get_hosts();
        // MULTIPLE CAPTURE (SENDER -> ALL CAPTURES)
        let recipient_ips = Vec::from(&HOST_ADDRESSES[1..]);
        let test2 = super::rip_large_network(recipient_ips.clone());

        assert_eq!(
            test2.await,
            super::ExitStatus::Status(recipient_ips.len() as u32)
        );
    }
}
