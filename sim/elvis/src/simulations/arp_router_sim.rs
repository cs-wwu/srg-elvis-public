use crate::applications::{ArpRouter, Capture, SendMessage};
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

const IPS: [Ipv4Address; 6] = [
    Ipv4Address::new([123, 45, 67, 89]),
    Ipv4Address::new([123, 45, 67, 90]),
    Ipv4Address::new([123, 45, 67, 91]),
    Ipv4Address::new([123, 45, 67, 92]),
    Ipv4Address::new([123, 45, 67, 93]),
    Ipv4Address::new([123, 45, 67, 94]),
];

const ROUTER1_IPS: [Ipv4Address; 4] = [
    Ipv4Address::new([123, 45, 76, 92]),
    Ipv4Address::new([123, 45, 76, 93]),
    Ipv4Address::new([123, 45, 76, 94]),
    Ipv4Address::new([123, 45, 76, 95]),
];

const ROUTER2_IPS: [Ipv4Address; 4] = [
    Ipv4Address::new([123, 45, 66, 92]),
    Ipv4Address::new([123, 45, 66, 93]),
    Ipv4Address::new([123, 45, 66, 94]),
    Ipv4Address::new([123, 45, 66, 95]),
];

pub fn build_ip_table(addresses: &[Ipv4Address]) -> IpTable<Recipient> {
    let mut ip_table = IpTable::<Recipient>::new();
    for entry in addresses.iter().enumerate() {
        ip_table.add_direct(*(entry.1), Recipient::new(entry.0 as u32, None));
    }
    ip_table
}

pub fn build_capture(network: Arc<Network>, address: Ipv4Address, exit_status: u32) -> Machine {
    new_machine![
        Udp::new(),
        Ipv4::new(Default::default()),
        Pci::new([network]),
        Arp::basic(),
        Capture::new(
            Endpoint {
                address,
                port: 0xbeef,
            },
            1,
        )
        .exit_status(exit_status)
    ]
}

/* KEY
      o or (n)   :  network

    - or | or /  :  connection
*/

/*routes packet from destination 0 to one of the given destinations 1,2,3

             /-(1)- 1
            |
    0 -(0)- R -(2)- 2
            |
             \-(3)- 3
*/
#[allow(dead_code)]
pub async fn arp_router_single(destination: Ipv4Address) -> ExitStatus {
    // Setting route to none sets the destination ip to the destination
    // ip in the received packet
    let router_table: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (IPS[0], (None, 0)),
        (IPS[1], (None, 1)),
        (IPS[2], (None, 2)),
        (IPS[3], (None, 3)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..4).map(|_| Network::basic()).collect();

    let ip_table = build_ip_table(&ROUTER1_IPS);

    let send_message = SendMessage::new(
        vec![Message::new(b"Hello World!")],
        Endpoint {
            address: destination,
            port: 0xbeef,
        },
    );

    let machines = vec![
        // send message
        new_machine![
            Udp::new(),
            Ipv4::new([(IPS[0], Recipient::new(0, None))].into_iter().collect(),),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER1_IPS[0]
                }
            ),
        ],
        // machine representing our router
        new_machine![
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone(),
                networks[3].clone(),
            ]),
            Ipv4::new(ip_table.clone()),
            Arp::basic(),
            ArpRouter::new(router_table, ROUTER1_IPS.to_vec())
        ],
        // capture for destination 1
        build_capture(networks[1].clone(), IPS[1], 1),
        // capture for destination 2
        build_capture(networks[2].clone(), IPS[2], 2),
        // capture for destination 3
        build_capture(networks[3].clone(), IPS[3], 3),
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(2)).await
}

/*  routes packet from destination 0 to one of the given destinations 1,2,3,4
                  1
                  |
 0 ---(0)--- R --(1)--2
             |    |
            (2)   3
             |
             4
*/
#[allow(dead_code)]
pub async fn arp_router_single2(destination: Ipv4Address) -> ExitStatus {
    let router_table: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (IPS[0], (None, 0)),
        (IPS[1], (None, 1)),
        (IPS[2], (None, 1)),
        (IPS[3], (None, 1)),
        (IPS[4], (None, 2)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..3).map(|_| Network::basic()).collect();
    let ip_table = build_ip_table(&ROUTER1_IPS);

    let send_message = SendMessage::new(
        vec![Message::new(b"Hello World!")],
        Endpoint {
            address: destination,
            port: 0xbeef,
        },
    );

    let machines = vec![
        // send message
        new_machine![
            Udp::new(),
            Ipv4::new([(IPS[0], Recipient::new(0, None))].into_iter().collect(),),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER1_IPS[0]
                }
            ),
        ],
        // machine representing our router
        new_machine![
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone()
            ]),
            Ipv4::new(ip_table.clone()),
            Arp::basic(),
            ArpRouter::new(router_table, ROUTER1_IPS.to_vec())
        ],
        build_capture(networks[1].clone(), IPS[1], 1),
        build_capture(networks[1].clone(), IPS[2], 2),
        build_capture(networks[1].clone(), IPS[3], 3),
        build_capture(networks[2].clone(), IPS[4], 4),
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(2)).await
}

/* routes packet from source 0 to one of the given destinations 1,2,3,4,5

        (1) -- 1    (4)- 2
         |           |
 0 -(0)- R1 --(2)--- R2 -(5)- 3
         |
        (3) -- 4
         |
         5
*/
#[allow(dead_code)]
pub async fn arp_router_multi(destination: Ipv4Address) -> ExitStatus {
    let router_table_1: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (IPS[0], (None, 0)),
        (IPS[1], (None, 1)),
        (IPS[2], (Some(ROUTER2_IPS[0]), 2)),
        (IPS[3], (Some(ROUTER2_IPS[0]), 2)),
        (IPS[4], (None, 3)),
        (IPS[5], (None, 3)),
    ]
    .into_iter()
    .collect();

    let router_table_2: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (IPS[0], (Some(ROUTER1_IPS[2]), 0)),
        (IPS[1], (Some(ROUTER1_IPS[2]), 0)),
        (IPS[2], (None, 1)),
        (IPS[3], (None, 2)),
        (IPS[4], (Some(ROUTER1_IPS[2]), 0)),
        (IPS[5], (Some(ROUTER1_IPS[2]), 0)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..6).map(|_| Network::basic()).collect();
    let ip_table_1 = build_ip_table(&ROUTER1_IPS);
    let ip_table_2 = build_ip_table(&ROUTER2_IPS);

    let send_message = SendMessage::new(
        vec![Message::new(b"Hello World!")],
        Endpoint {
            address: destination,
            port: 0xbeef,
        },
    );

    let machines = vec![
        // send message
        new_machine![
            Udp::new(),
            Ipv4::new([(IPS[0], Recipient::new(0, None))].into_iter().collect(),),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER1_IPS[0]
                }
            ),
        ],
        // Routers
        new_machine![
            Pci::new([
                networks[0].clone(),
                networks[1].clone(),
                networks[2].clone(),
                networks[3].clone()
            ]),
            Ipv4::new(ip_table_1),
            Arp::basic(),
            ArpRouter::new(router_table_1, ROUTER1_IPS.to_vec())
        ],
        new_machine![
            Pci::new([
                networks[2].clone(),
                networks[4].clone(),
                networks[5].clone(),
            ]),
            Ipv4::new(ip_table_2),
            Arp::basic(),
            ArpRouter::new(router_table_2, ROUTER2_IPS.to_vec())
        ],
        // Destinations
        build_capture(networks[1].clone(), IPS[1], 1),
        build_capture(networks[4].clone(), IPS[2], 2),
        build_capture(networks[5].clone(), IPS[3], 3),
        build_capture(networks[3].clone(), IPS[4], 4),
        build_capture(networks[3].clone(), IPS[5], 5),
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(2)).await
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn arp_router_single() {
        let test1 = super::arp_router_single(super::IPS[1]);
        let test2 = super::arp_router_single(super::IPS[2]);
        let test3 = super::arp_router_single(super::IPS[3]);

        assert_eq!(test1.await, super::ExitStatus::Status(1));
        assert_eq!(test2.await, super::ExitStatus::Status(2));
        assert_eq!(test3.await, super::ExitStatus::Status(3));
    }

    #[tokio::test]
    async fn arp_router_single2() {
        let test1 = super::arp_router_single2(super::IPS[1]);
        let test2 = super::arp_router_single2(super::IPS[2]);
        let test3 = super::arp_router_single2(super::IPS[3]);
        let test4 = super::arp_router_single2(super::IPS[4]);

        assert_eq!(test1.await, super::ExitStatus::Status(1));
        assert_eq!(test2.await, super::ExitStatus::Status(2));
        assert_eq!(test3.await, super::ExitStatus::Status(3));
        assert_eq!(test4.await, super::ExitStatus::Status(4));
    }

    #[tokio::test]
    async fn arp_router_multi() {
        let test1 = super::arp_router_multi(super::IPS[1]);
        let test2 = super::arp_router_multi(super::IPS[2]);
        let test3 = super::arp_router_multi(super::IPS[3]);
        let test4 = super::arp_router_multi(super::IPS[4]);
        let test5 = super::arp_router_multi(super::IPS[5]);

        assert_eq!(test1.await, super::ExitStatus::Status(1));
        assert_eq!(test2.await, super::ExitStatus::Status(2));
        assert_eq!(test3.await, super::ExitStatus::Status(3));
        assert_eq!(test4.await, super::ExitStatus::Status(4));
        assert_eq!(test5.await, super::ExitStatus::Status(5));
    }
}
