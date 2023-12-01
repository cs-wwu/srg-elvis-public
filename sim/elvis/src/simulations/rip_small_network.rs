<<<<<<< HEAD
use crate::applications::{rip::rip_router::RipRouter, ArpRouter, Capture, SendMessage};
use elvis_core::{
    machine::PciSlot,
=======
use crate::applications::{rip::rip_router::{RipRouter, RoutingTable}, ArpRouter, MultiCapture, Counter, SendMessage};
use elvis_core::{
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
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
<<<<<<< HEAD
use std::{sync::Arc, sync::RwLock, time::Duration};
=======
use std::{sync::Arc, time::Duration};
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)

// Routes a message from a SENDER machine to all CAP (capture) machines
// ----------------------------------------------------------------------
// KEY
// Router: R(x)
// Machine: M(x)
// Network: N(x)
// where x is an identification
// ----------------------------------------------------------------------
// Networks:
<<<<<<< HEAD
// N1 = 10.0.1.0/24
// N2 = 10.0.2.0/24
// N3 = 10.0.3.0/24
// N4 = 10.0.4.0/24
// ----------------------------------------------------------------------
// M(SENDER) -- N(N1) -- R(RIP1) -- N(N2) -- R(RIP2) -- N(N3) -- M(CAP1)
//                                  |
//                                R(RIP3)
//                                  |
//                                N(N4)
//                                  |
//                                M(CAP2)

const MESSAGE_PORT: u16 = 0xdeeb;

const HOST_ADDRESSES: [Ipv4Address; 3] = [
    // SENDER
    Ipv4Address::new([10, 0, 1, 100]),
    // CAPTURE 1
    Ipv4Address::new([10, 0, 3, 100]),
    // CAPTURE 2
    Ipv4Address::new([10, 0, 4, 100]),
=======
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
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
];

const ROUTER_1_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N1
<<<<<<< HEAD
    Ipv4Address::new([10, 0, 1, 1]),
    // Interface to N2
    Ipv4Address::new([10, 0, 2, 1]),
=======
    Ipv4Address::new([10, 0, 0, 2]),
    // Interface to N2
    Ipv4Address::new([10, 0, 0, 9]),
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
];

const ROUTER_2_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N2
<<<<<<< HEAD
    Ipv4Address::new([10, 0, 2, 2]),
    // Interface to N3
    Ipv4Address::new([10, 0, 3, 2]),
];

const ROUTER_3_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N2
    Ipv4Address::new([10, 0, 2, 3]),
    // Interface to N4
    Ipv4Address::new([10, 0, 4, 3]),
=======
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
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
];


pub fn create_capture(
    ip: Ipv4Address,
    subnet: SubnetInfo,
    network: Arc<Network>,
<<<<<<< HEAD
    status_ref: Option<Arc<RwLock<u32>>>, //
    exit_status: u32,
) -> Machine {
=======
    multicapture_counter: Arc<Counter>,
) -> Machine {
<<<<<<< HEAD
    new_machine![
        Pci::new([network]),
        Arp::new().preconfig_subnet(ip, subnet),
        Ipv4::new(Default::default()),
        Udp::new(),
        MultiCapture::new(Endpoint::new(ip, MESSAGE_PORT), multicapture_counter)
    ]
=======
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    if let Some(status_ref) = status_ref {
        new_machine![
            Pci::new([network]),
            Arp::new().preconfig_subnet(ip, subnet),
            Ipv4::new(Default::default()),
            Udp::new(),
            Capture::new(Endpoint::new(ip, MESSAGE_PORT), 1)
                .with_atomic_status(status_ref, exit_status)
        ]
    } else {
        new_machine![
            Pci::new([network]),
            Arp::new().preconfig_subnet(ip, subnet),
            Ipv4::new(Default::default()),
            Udp::new(),
            Capture::new(Endpoint::new(ip, MESSAGE_PORT), 1)
                .exit_status(exit_status)
        ]
    }
<<<<<<< HEAD
=======
>>>>>>> 3fd40a49 (Added functionality to send message to multiple machines (from gab) and reworked some things)
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
}

pub fn create_router(
    // Attached networks
    networks: impl IntoIterator<Item = Arc<Network>>,
    // Router's interface IPs
    interface_ips: &[Ipv4Address],
<<<<<<< HEAD
    // Neighboring end devices
    routing_table: IpTable<(Option<Ipv4Address>, PciSlot)>,
=======
<<<<<<< HEAD
    // Routing table to end devices
    routing_table: RoutingTable,
=======
    // Neighboring end devices
    routing_table: IpTable<(Option<Ipv4Address>, PciSlot)>,
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
) -> Machine {
    // IPs are mapped to interfaces/pcis (of networks) based on their order
    // E.g. the first address in interface_ips will be the ip of the first pci interface

<<<<<<< HEAD
=======
<<<<<<<< HEAD:sim/elvis/src/simulations/rip_large_network.rs
    let mut interfaces = IpTable::<Recipient>::new();
    for (pci_slot, addr) in interface_ips.iter().enumerate() {
        //println!("slot: {}", pci_slot);
        interfaces.add_direct(*addr, Recipient::new(pci_slot as u32, None));
    }
========
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    let interfaces = interface_ips.into();
    // let mut interfaces = IpTable::<Recipient>::new();
    // for (pci_slot, addr) in interface_ips.iter().enumerate() {
    //     interfaces.add_direct(*addr, Recipient::new(pci_slot as u32, None));
    // }
<<<<<<< HEAD

=======
>>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion):sim/elvis/src/simulations/rip_small_network.rs

<<<<<<< HEAD
<<<<<<< HEAD
=======
    // let mut routing_table = IpTable::<(Option<Ipv4Address>, PciSlot)>::new();
    // for (pci_slot, neighbor_ip) in neighbors.iter().enumerate() {
    //     println!("ip: {}", neighbor_ip);
    //     println!("slot: {}", pci_slot);
    //     routing_table.add_direct(*neighbor_ip, (None, pci_slot as u32));
    // }

>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
=======
>>>>>>> 88fbfdc2 (added endpoint list creation from ip list ... tried to create interface ip table (not working yet))
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    new_machine![
        Pci::new(networks),
        Arp::new(),
        Ipv4::new(interfaces),
        Udp::new(),
        ArpRouter::new(routing_table, Vec::from(interface_ips)),
        RipRouter::new(Vec::from(interface_ips)),
    ]
<<<<<<< HEAD
}

pub fn gen_capture_machines(networks: Vec<Arc<Network>>) -> Vec<Machine> {
    vec![
        // Capture 1
        create_capture(
            // Address of machine
            HOST_ADDRESSES[1],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(24),
                default_gateway: ROUTER_2_INTERFACES[1],
            },
            // Attached to network 3
            networks[2].clone(),
            None,
            // Exit status on recieve
            1,
        ),
        // Capture 2
        create_capture(
            HOST_ADDRESSES[2],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(24),
                default_gateway: ROUTER_3_INTERFACES[1],
            },
            // Attached to network 4
            networks[3].clone(),
            None,
            2,
        )
    ]
}

pub fn gen_capture_machines_status(
    networks: Vec<Arc<Network>>,
    status: Arc<RwLock<u32>>,
) -> Vec<Machine> {
    vec![
        // Capture 1
        create_capture(
            // Address of machine
            HOST_ADDRESSES[1],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(24),
                default_gateway: ROUTER_2_INTERFACES[1],
            },
            // Attached network to network 3
            networks[2].clone(),
            Some(status.clone()),
            // Exit status on recieve
            1,
        ),
        // Capture 2
        create_capture(
            HOST_ADDRESSES[2],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(24),
                default_gateway: ROUTER_3_INTERFACES[1],
            },
            // Attached to network 4
            networks[3].clone(),
            Some(status.clone()),
            2,
        )
    ]
=======

    // ArpRouter::new((interface_ips, pci), optional_routing_table);
    // RipRouter::new()
    //     .broadcast_network(subnet)
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
}

pub async fn rip_small_network(
    capture_ips: Vec<Ipv4Address>,
<<<<<<< HEAD
    status_capture: Option<Arc<RwLock<u32>>>,
=======
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
) -> ExitStatus {
    // Create 4 basic networks
    // Network::basic() :   mtu = maximum packet size;
    //                      throughput = amount of data successfully transmitted from x to y in a fixed amount of time
    //                      latency = simulated packet transit time
<<<<<<< HEAD
    let networks: Vec<Arc<Network>> = (0..4).map(|_| Network::basic()).collect();
=======
    let networks: Vec<Arc<Network>> = (0..7).map(|_| Network::basic()).collect();
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)

    // Create a lists of endpoints for capture machines
    let endpoints: Vec<Endpoint> = Endpoint::new_vec(capture_ips, MESSAGE_PORT);
    // capture_ips
    //     .iter()
    //     .for_each(|recipient_ip| endpoints.push(Endpoint::new(*recipient_ip, MESSAGE_PORT)));

<<<<<<< HEAD
    // Only sending message to CAP2
    let message = SendMessage::with_endpoints(vec![Message::new(b"Yahoo")], endpoints)
        .delay(Duration::from_secs(5));
=======
<<<<<<< HEAD
    // Number of recipients = numebr of capture_ips
    let multicapture_counter = Counter::new(capture_ips.len() as u32);

    // Only sending message to CAP3
    let message = SendMessage::with_endpoints(vec![Message::new(b"Yahoo")], endpoints)
        .delay(Duration::from_secs(3));
=======
    // Only sending message to CAP2
    let message = SendMessage::with_endpoints(vec![Message::new(b"Yahoo")], endpoints)
<<<<<<< HEAD
        .delay(Duration::from_secs(2));
>>>>>>> 3fd40a49 (Added functionality to send message to multiple machines (from gab) and reworked some things)
=======
        .delay(Duration::from_secs(5));
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)

    // Everything is a machine
    let mut end_devices = vec![
        // SENDER MACHINE
        new_machine![
            // Pci attached to network 1
            Pci::new([networks[0].clone()]),
            // Host IP configuration
            Arp::new().preconfig_subnet(
                // Sender IP
                HOST_ADDRESSES[0],
                SubnetInfo {
<<<<<<< HEAD
                    mask: Ipv4Mask::from_bitcount(24),
=======
                    mask: Ipv4Mask::from_bitcount(30),
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
                    default_gateway: ROUTER_1_INTERFACES[0]
                }
            ),
            // IPv4 protocol intended to send message from ip HOST_ADDRESSES[0] out Pci slot 0
            Ipv4::new(IpTable::from_iter(
                [(HOST_ADDRESSES[0], Recipient::new(0, None))].into_iter()
            )),
            // Using transport protocol: udp
            Udp::new(),
<<<<<<< HEAD
            message.local_ip(HOST_ADDRESSES[0])
        ],
    ];

    let captures = if let Some(status_cap) = status_capture {
        gen_capture_machines_status(networks.clone(), status_cap)
    } else {
        gen_capture_machines(networks.clone())
    };
=======
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
            multicapture_counter.clone()
        ),
        // Capture 2
        create_capture(
            HOST_ADDRESSES[2],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(29),
                default_gateway: ROUTER_4_INTERFACES[1],
            },
            networks[5].clone(),
            multicapture_counter.clone()
        ),
        // Capture 3
        create_capture(
            HOST_ADDRESSES[3],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(29),
                default_gateway: ROUTER_4_INTERFACES[1],
            },
            networks[5].clone(),
            multicapture_counter.clone()
        ),
        // Capture 4
        create_capture(
            HOST_ADDRESSES[4],
            SubnetInfo {
                mask: Ipv4Mask::from_bitcount(30),
                default_gateway: ROUTER_5_INTERFACES[1],
            },
            networks[6].clone(),
            multicapture_counter.clone()
        ),
    ];
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    end_devices.extend(captures);

    let r_table_1: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (HOST_ADDRESSES[0],(None, 0)),
    ]
    .into_iter()
    .collect();

    let r_table_2: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (HOST_ADDRESSES[1],(None, 1)),
    ]
    .into_iter()
    .collect();

    let r_table_3: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (HOST_ADDRESSES[2],(None, 1)),
    ]
    .into_iter()
    .collect();
    let mut routers = vec![
        // RIP 1
        create_router(
<<<<<<< HEAD
            // Connected networks (1 & 2)
            [networks[0].clone(), networks[1].clone()],
            &ROUTER_1_INTERFACES,
            // Connected hosts (Sender: 0)
            r_table_1,
        ),
        // RIP 2
        create_router(
            // Connected networks (2 & 3)
            [networks[1].clone(), networks[2].clone()],
            &ROUTER_2_INTERFACES,
            // Connected hosts (CAP1)
            r_table_2,
        ),
        // RIP 3
        create_router(
            // Connected networks (2 & 4)
            [
                networks[1].clone(),
                networks[3].clone(),
            ],
            &ROUTER_3_INTERFACES,
            // Connected hosts (CAP2)
            r_table_3,
        )
=======
            // Connected networks
            [networks[0].clone(), networks[1].clone()],
            &ROUTER_1_INTERFACES,
<<<<<<< HEAD
            // Connected hosts
            [(HOST_ADDRESSES[0], (None, 1))].into_iter().collect(),
=======
            // Connected hosts (Sender: 0)
            r_table_1,
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
        ),
        // RIP 2
        create_router(
            [networks[1].clone(), networks[2].clone()],
            &ROUTER_2_INTERFACES,
<<<<<<< HEAD
            [(HOST_ADDRESSES[1], (None, 1))].into_iter().collect(),
=======
            // Connected hosts (CAP1)
            r_table_2,
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
        ),
        // RIP 3
        create_router(
            [
                networks[1].clone(),
                networks[3].clone(),
                networks[4].clone(),
            ],
            &ROUTER_3_INTERFACES,
<<<<<<< HEAD
            // RIP router is connected to no hosts
            RoutingTable::new(),
        ),
        // RIP 4
        create_router(
            [networks[3].clone(), networks[5].clone()],
            &ROUTER_4_INTERFACES,
            [(HOST_ADDRESSES[2], (None, 1)), (HOST_ADDRESSES[3], (None, 1))].into_iter().collect(),
        ),
        // RIP 5
        create_router(
            [networks[4].clone(), networks[6].clone()],
            &ROUTER_5_INTERFACES,
            [(HOST_ADDRESSES[4], (None, 1))].into_iter().collect(),
        ),
=======
            // Connected hosts (CAP2)
            r_table_3,
        )
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    ];

    routers.extend(end_devices);
    let machines = routers;

    run_internet_with_timeout(&machines, Duration::from_secs(10)).await
}


#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
<<<<<<< HEAD
=======
<<<<<<< HEAD:sim/elvis/src/simulations/rip_large_network.rs
    async fn rip_large_network() {
        // SINGLE CAPTURE (SENDER -> CAPTURE3)
        let recipient_ips = Vec::from([HOST_ADDRESSES[3]]);
        let test1 = super::rip_large_network(recipient_ips.clone());
=======
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    async fn rip_small_network() {
        // SINGLE CAPTURE (SENDER -> CAPTURE2)
        let recipient_ips = Vec::from([HOST_ADDRESSES[2]]);
        let test1 = super::rip_small_network(recipient_ips, None);
<<<<<<< HEAD

        // Message should reach capture 2 (and no other)
        assert_eq!(test1.await, super::ExitStatus::Status(2));
=======
>>>>>>> 9bf657f7 (Changed sim file name, implemented ips to vec<recicpient> conversion):sim/elvis/src/simulations/rip_small_network.rs

        // Message should reach capture 3 (and no other)
        assert_eq!(test1.await, super::ExitStatus::Status(recipient_ips.len() as u32));
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
    }

    #[tokio::test]
    async fn rip_large_network_all() {
        // MULTIPLE CAPTURE (SENDER -> ALL CAPTURES)
<<<<<<< HEAD
        let recipient_ips = Vec::from(HOST_ADDRESSES);
        let status = Arc::new(RwLock::new(0));
        let test2 = super::rip_small_network(recipient_ips, Some(status.clone()));

        assert_eq!(test2.await, super::ExitStatus::Exited);
        assert_eq!(*status.read().unwrap(), 1 + 2 );
    }
}
=======
<<<<<<< HEAD:sim/elvis/src/simulations/rip_large_network.rs
        let recipient_ips = Vec::from(&HOST_ADDRESSES[1..]);
        let test2 = super::rip_large_network(recipient_ips.clone());
=======
        let recipient_ips = Vec::from(HOST_ADDRESSES);
        let status = Arc::new(RwLock::new(0));
        let test2 = super::rip_small_network(recipient_ips, Some(status.clone()));
>>>>>>> 9bf657f7 (Changed sim file name, implemented ips to vec<recicpient> conversion):sim/elvis/src/simulations/rip_small_network.rs

<<<<<<< HEAD
<<<<<<< HEAD
        assert_eq!(test2.await, super::ExitStatus::Status(recipient_ips.len() as u32));
=======
        assert_eq!(test2.await, super::ExitStatus::TimedOut);
=======
        assert_eq!(test2.await, super::ExitStatus::Exited);
>>>>>>> 248d104f (PCI Slots in routing table changed. Sim working)
        assert_eq!(*status.read().unwrap(), 1 + 2 );
>>>>>>> 3fd40a49 (Added functionality to send message to multiple machines (from gab) and reworked some things)
    }
}
>>>>>>> cbb5e19e (Changed sim file name, implemented ips to vec<recicpient> conversion)
