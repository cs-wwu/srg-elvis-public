use crate::applications::{rip::rip_router::RipRouter, arp_router::RoutingTable, ArpRouter, MultiCapture, Counter, SendMessage};
use elvis_core::{
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
];

const ROUTER_1_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N1
    Ipv4Address::new([10, 0, 1, 1]),
    // Interface to N2
    Ipv4Address::new([10, 0, 2, 1]),
];

const ROUTER_2_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N2
    Ipv4Address::new([10, 0, 2, 2]),
    // Interface to N3
    Ipv4Address::new([10, 0, 3, 2]),
];

const ROUTER_3_INTERFACES: [Ipv4Address; 2] = [
    // Interface to N2
    Ipv4Address::new([10, 0, 2, 3]),
    // Interface to N4
    Ipv4Address::new([10, 0, 4, 3]),
];

pub fn create_capture(
    ip: Ipv4Address,
    subnet: SubnetInfo,
    network: Arc<Network>,
    multicapture_counter: Arc<Counter>,
) -> Machine {
    new_machine![
        Pci::new([network]),
        Arp::new().preconfig_subnet(ip, subnet),
        Ipv4::new(Default::default()),
        Udp::new(),
        MultiCapture::new(Endpoint::new(ip, MESSAGE_PORT), multicapture_counter)
    ]
}

pub fn create_router(
    // Attached networks
    networks: impl IntoIterator<Item = Arc<Network>>,
    // Router's interface IPs
    interface_ips: &[Ipv4Address],
    // Neighboring end devices
    routing_table: RoutingTable,
) -> Machine {
    // IPs are mapped to interfaces/pcis (of networks) based on their order
    // E.g. the first address in interface_ips will be the ip of the first pci interface

    let interfaces: IpTable<Recipient> = interface_ips.into();

    new_machine![
        Pci::new(networks),
        Arp::new(),
        Ipv4::new(interfaces),
        Udp::new(),
        ArpRouter::from_table(routing_table),
        RipRouter::new(),
    ]
}

pub async fn rip_small_network(
    capture_ips: Vec<Ipv4Address>,
) -> ExitStatus {
    // Create 4 basic networks
    // Network::basic() :   mtu = maximum packet size;
    //                      throughput = amount of data successfully transmitted from x to y in a fixed amount of time
    //                      latency = simulated packet transit time
    let networks: Vec<Arc<Network>> = (0..4).map(|_| Network::basic()).collect();

    // Create a lists of endpoints for capture machines
    let endpoints: Vec<Endpoint> = Endpoint::new_vec(&capture_ips, MESSAGE_PORT);

    // Number of recipients = numebr of capture_ips
    let multicapture_counter = Counter::new(capture_ips.len() as u32);
    
    // Create SendMessage application with multiple endpoints
    let message = SendMessage::with_endpoints(vec![Message::new(b"Yahoo")], endpoints)
        .delay(Duration::from_secs(5));

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
                    mask: Ipv4Mask::from_bitcount(24),
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
                mask: Ipv4Mask::from_bitcount(24), 
                default_gateway: ROUTER_2_INTERFACES[1]
            }, 
            // Attached network
            networks[2].clone(), 
            // Multicapture counter and status
            multicapture_counter.clone()
        ),
        // Capture 2
        create_capture(
            // Address of machine
            HOST_ADDRESSES[2],
            SubnetInfo { 
                mask: Ipv4Mask::from_bitcount(24), 
                default_gateway: ROUTER_3_INTERFACES[1],
            },
            // Attached network
            networks[3].clone(),
            // Multicapture counter and status
            multicapture_counter.clone()
        ),
    ];

    end_devices.extend(captures);

    let mut routers = vec![
        // RIP 1
        create_router(
            // Connected networks (1 & 2)
            [networks[0].clone(), networks[1].clone()],
            &ROUTER_1_INTERFACES,
            // Connected hosts (Sender: 0)
            [(HOST_ADDRESSES[0],(None, 0))].into_iter().collect(),
        ),
        // RIP 2
        create_router(
            // Connected networks (2 & 3)
            [networks[1].clone(), networks[2].clone()],
            &ROUTER_2_INTERFACES,
            // Connected hosts (CAP1)
            [(HOST_ADDRESSES[1],(None, 1))].into_iter().collect(),
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
            [(HOST_ADDRESSES[2],(None, 1))].into_iter().collect(),
        )
    ];

    routers.extend(end_devices);
    let machines = routers;

    run_internet_with_timeout(&machines, Duration::from_secs(10)).await
}


#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn rip_small_network() {
        // SINGLE CAPTURE (SENDER -> CAPTURE2)
        let recipient_ips = Vec::from([HOST_ADDRESSES[2]]);
        let test1 = super::rip_small_network(recipient_ips.clone());

        // Message should reach capture 2 (and no other)
        assert_eq!(test1.await, super::ExitStatus::Status(recipient_ips.len() as u32));
    }

    #[tokio::test]
    async fn rip_small_network_all() {
        // MULTIPLE CAPTURE (SENDER -> ALL CAPTURES)
        let recipient_ips = Vec::from(HOST_ADDRESSES);
        let test2 = super::rip_small_network(recipient_ips.clone());

        assert_eq!(test2.await, super::ExitStatus::Status(recipient_ips.len() as u32));
    }
}