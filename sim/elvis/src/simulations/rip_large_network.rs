use crate::applications::{rip::rip_router::RipRouter, ArpRouter, MultiCapture, Counter, SendMessage};
use elvis_core::{
    machine::PciSlot,
    new_machine,
    protocols::{
        arp::subnetting::{Ipv4Mask, SubnetInfo},
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

const HOST_ADDRESSES: [Ipv4Address; 5] = [
    // SENDER
    Ipv4Address::new([10, 0, 0, 1]),
    // CAPTURE 1
    Ipv4Address::new([10, 0, 0, 6]),
    // CAPTURE 2
    Ipv4Address::new([10, 0, 0, 26]),
    // CAPTURE 3
    Ipv4Address::new([10, 0, 0, 27]),
    // CAPTURE 4
    Ipv4Address::new([10, 0, 0, 34]),
];

const ROUTER_1_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N1
    Ipv4Address::new([10, 0, 0, 2]),
    // Interface to N2
    Ipv4Address::new([10, 0, 0, 9]),
];

const ROUTER_2_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N2
    Ipv4Address::new([10, 0, 0, 10]),
    // Interface to N3
    Ipv4Address::new([10, 0, 0, 5]),
];

const ROUTER_3_INTERFACES: [Ipv4Address; 3] = [
    // Interface to N2
    Ipv4Address::new([10, 0, 0, 11]),
    // Interface to N4
    Ipv4Address::new([10, 0, 0, 17]),
    // Interface to N5
    Ipv4Address::new([10, 0, 0, 21]),
];

const ROUTER_4_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N4
    Ipv4Address::new([10, 0, 0, 18]),
    // Interface to N6
    Ipv4Address::new([10, 0, 0, 25]),
];

const ROUTER_5_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N5
    Ipv4Address::new([10, 0, 0, 22]),
    // Interface to N7
    Ipv4Address::new([10, 0, 0, 33]),
];

pub fn create_capture(
    ip: Ipv4Address,
    subnet: SubnetInfo,
    network: Arc<Network>,
    multicapture_counter: Arc<Counter>,
    status: u32,
) -> Machine {
    new_machine![
        Pci::new([network]),
        Arp::basic().preconfig_subnet(ip, subnet),
        Ipv4::new(Default::default()),
        Udp::new(),
        MultiCapture::new(Endpoint::new(ip, MESSAGE_PORT), multicapture_counter).exit_status(status)
    ]
}

pub fn create_router(
    // Attached networks
    networks: impl IntoIterator<Item = Arc<Network>>,
    // Router's interface IPs
    interface_ips: &[Ipv4Address],
    // Neighboring end devices
    neighbors: Vec<Ipv4Address>,
) -> Machine {
    // IPs are mapped to interfaces/pcis (of networks) based on their order
    // E.g. the first address in interface_ips will be the ip of the first pci interface

    let mut interfaces = IpTable::<Recipient>::new();
    for (pci_slot, addr) in interface_ips.iter().enumerate() {
        interfaces.add_direct(*addr, Recipient::new(pci_slot as u32, None));
    }

    let mut routing_table = IpTable::<(Option<Ipv4Address>, PciSlot)>::new();
    for (pci_slot, neighbor_ip) in neighbors.iter().enumerate() {
        routing_table.add_direct(*neighbor_ip, (None, pci_slot as u32));
    }

    new_machine![
        Pci::new(networks),
        Arp::basic(),
        Ipv4::new(interfaces),
        Udp::new(),
        ArpRouter::new(routing_table, Vec::from(interface_ips)),
        RipRouter::new(Vec::from(interface_ips)),
    ]
}

pub async fn rip_large_network(
    capture_ips: Vec<Ipv4Address>,
) -> ExitStatus {
    // Create 7 basic networks
    // Network::basic() :   mtu = maximum packet size;
    //                      throughput = amount of data successfully transmitted from x to y in a fixed amount of time
    //                      latency = simulated packet transit time
    let networks: Vec<Arc<Network>> = (0..7).map(|_| Network::basic()).collect();

    // Create a lists of endpoints for capture machines
    let mut endpoints = Vec::new();
    capture_ips
        .iter()
        .for_each(|recipient_ip| endpoints.push(Endpoint::new(*recipient_ip, MESSAGE_PORT)));

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
            Arp::basic().preconfig_subnet(
                // Sender IP
                HOST_ADDRESSES[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(30),
                    default_gateway: ROUTER_1_INTERFACES[0]
                }
            ),
            // IPv4 protocol intended to send message from ip HOST_ADDRESSES[0] out Pci slot 0
            Ipv4::new(IpTable::from_iter(
                [(HOST_ADDRESSES[0], Recipient::new(0, None))].into_iter()
            )),
            // Using transport protocol: udp
            Udp::new(),
            message.local_ip(HOST_ADDRESSES[0]),
            MultiCapture::new(Endpoint::new(HOST_ADDRESSES[0], MESSAGE_PORT), multicapture_counter.clone())
        ],
    ];

    let captures = vec![
        // Capture 1
        create_capture(
            // Address of machine
            HOST_ADDRESSES[1],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(30),
                default_gateway: ROUTER_2_INTERFACES[1],
            },
            // Attached network
            networks[2].clone(),
            // Multicapture counter and status
            multicapture_counter.clone(),
            1
        ),
        // Capture 2
        create_capture(
            HOST_ADDRESSES[2],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(29),
                default_gateway: ROUTER_4_INTERFACES[1],
            },
            networks[5].clone(),
            multicapture_counter.clone(),
            2,
        ),
        // Capture 3
        create_capture(
            HOST_ADDRESSES[3],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(29),
                default_gateway: ROUTER_4_INTERFACES[1],
            },
            networks[5].clone(),
            multicapture_counter.clone(),
            3,
        ),
        // Capture 4
        create_capture(
            HOST_ADDRESSES[4],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(30),
                default_gateway: ROUTER_5_INTERFACES[1],
            },
            networks[6].clone(),
            multicapture_counter.clone(),
            4,
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
            Vec::from([HOST_ADDRESSES[0]]),
        ),
        // RIP 2
        create_router(
            [networks[1].clone(), networks[2].clone()],
            &ROUTER_2_INTERFACES,
            Vec::from([HOST_ADDRESSES[1]]),
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
            Vec::new(),
        ),
        // RIP 4
        create_router(
            [networks[3].clone(), networks[5].clone()],
            &ROUTER_4_INTERFACES,
            Vec::from([HOST_ADDRESSES[2], HOST_ADDRESSES[3]]),
        ),
        // RIP 5
        create_router(
            [networks[4].clone(), networks[6].clone()],
            &ROUTER_5_INTERFACES,
            Vec::from([HOST_ADDRESSES[4]]),
        ),
    ];

    routers.extend(end_devices);
    let machines = routers;

    run_internet_with_timeout(&machines, Duration::from_secs(10)).await
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn rip_large_network() {
        // SINGLE CAPTURE (SENDER -> CAPTURE3)
        let recipient_ips = Vec::from([HOST_ADDRESSES[3]]);
        let test1 = super::rip_large_network(recipient_ips);

        // Message should reach capture 3 (and no other)
        assert_eq!(test1.await, super::ExitStatus::Status(3));
    }

    #[tokio::test]
    async fn rip_large_network_all() {
        // MULTIPLE CAPTURE (SENDER -> ALL CAPTURES)
        let recipient_ips = Vec::from(HOST_ADDRESSES);
        let test2 = super::rip_large_network(recipient_ips);

        assert_eq!(test2.await, super::ExitStatus::Status(1));
    }
}
