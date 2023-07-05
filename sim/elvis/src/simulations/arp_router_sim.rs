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

const ROUTER_IPS: [Ipv4Address; 2] = [
    Ipv4Address::new([123, 45, 76, 92]),
    Ipv4Address::new([123, 45, 76, 93]),
];

pub fn build_ip_table(router_table: &IpTable<(Ipv4Address, PciSlot)>) -> IpTable<Recipient> {
    let mut ip_table = IpTable::<Recipient>::new();
    for entry in router_table.iter() {
        ip_table.add(*entry.0, Recipient::new((entry.1).1, None));
    }
    ip_table
}

pub fn build_dest_table(address: Ipv4Address, slot: PciSlot) -> IpTable<Recipient> {
    [(address, Recipient::new(slot, None))]
        .into_iter()
        .collect()
}

pub fn build_capture(
    destination_table: IpTable<Recipient>,
    network: Arc<Network>,
    address: Ipv4Address,
    exit_status: u32,
) -> Machine {
    new_machine![
        Udp::new(),
        Ipv4::new(destination_table),
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
    let router_table: IpTable<(Ipv4Address, PciSlot)> = [
        (IPS[0], (IPS[0], 0)),
        (IPS[1], (IPS[1], 1)),
        (IPS[2], (IPS[2], 2)),
        (IPS[3], (IPS[3], 3)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..4).map(|_| Network::basic()).collect();
    let ip_table = build_ip_table(&router_table);
    let destination_table: Vec<_> = (0..4)
        .map(|index| build_dest_table(IPS[index], 0))
        .collect();

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
            Ipv4::new(
                [(destination, Recipient::new(0, None))]
                    .into_iter()
                    .collect(),
            ),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER_IPS[0]
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
            ArpRouter::new(router_table, ROUTER_IPS[0])
        ],
        // capture for destination 1
        build_capture(destination_table[1].clone(), networks[1].clone(), IPS[1], 1),
        // capture for destination 2
        build_capture(destination_table[2].clone(), networks[2].clone(), IPS[2], 2),
        // capture for destination 3
        build_capture(destination_table[3].clone(), networks[3].clone(), IPS[3], 3),
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
    let router_table: IpTable<(Ipv4Address, PciSlot)> = [
        (IPS[0], (IPS[0], 0)),
        (IPS[1], (IPS[1], 1)),
        (IPS[2], (IPS[2], 1)),
        (IPS[3], (IPS[3], 1)),
        (IPS[4], (IPS[4], 2)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..3).map(|_| Network::basic()).collect();
    let ip_table = build_ip_table(&router_table);
    let destination_table: Vec<_> = (0..5)
        .map(|index| build_dest_table(IPS[index], 0))
        .collect();

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
            Ipv4::new(
                [(destination, Recipient::new(0, None))]
                    .into_iter()
                    .collect(),
            ),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER_IPS[0]
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
            ArpRouter::new(router_table, ROUTER_IPS[0])
        ],
        build_capture(destination_table[1].clone(), networks[1].clone(), IPS[1], 1),
        build_capture(destination_table[2].clone(), networks[1].clone(), IPS[2], 2),
        build_capture(destination_table[3].clone(), networks[1].clone(), IPS[3], 3),
        build_capture(destination_table[4].clone(), networks[2].clone(), IPS[4], 4),
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
    let router_table_1: IpTable<(Ipv4Address, PciSlot)> = [
        (IPS[0], (IPS[0], 0)),
        (IPS[1], (IPS[1], 1)),
        (IPS[2], (ROUTER_IPS[1], 2)),
        (IPS[3], (ROUTER_IPS[1], 2)),
        (IPS[4], (IPS[4], 3)),
        (IPS[5], (IPS[5], 3)),
    ]
    .into_iter()
    .collect();

    let router_table_2: IpTable<(Ipv4Address, PciSlot)> = [
        (IPS[0], (ROUTER_IPS[0], 0)),
        (IPS[1], (ROUTER_IPS[0], 0)),
        (IPS[2], (IPS[2], 1)),
        (IPS[3], (IPS[3], 2)),
        (IPS[4], (ROUTER_IPS[0], 0)),
        (IPS[5], (ROUTER_IPS[0], 0)),
    ]
    .into_iter()
    .collect();

    let networks: Vec<_> = (0..6).map(|_| Network::basic()).collect();
    let ip_table_1 = build_ip_table(&router_table_1);
    let ip_table_2 = build_ip_table(&router_table_2);
    let destination_table: Vec<_> = (0..6)
        .map(|index| build_dest_table(IPS[index], 0))
        .collect();

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
            Ipv4::new(
                [(destination, Recipient::new(0, None))]
                    .into_iter()
                    .collect(),
            ),
            Pci::new([networks[0].clone()]),
            send_message.local_ip(IPS[0]),
            Arp::basic().preconfig_subnet(
                IPS[0],
                SubnetInfo {
                    mask: Ipv4Mask::from_bitcount(32),
                    default_gateway: ROUTER_IPS[0]
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
            Ipv4::new(ip_table_1.clone()),
            Arp::basic(),
            ArpRouter::new(router_table_1, ROUTER_IPS[0])
        ],
        new_machine![
            Pci::new([
                networks[2].clone(),
                networks[4].clone(),
                networks[5].clone(),
            ]),
            Ipv4::new(ip_table_2.clone()),
            Arp::basic(),
            ArpRouter::new(router_table_2, ROUTER_IPS[1])
        ],
        // Destinations
        build_capture(destination_table[1].clone(), networks[1].clone(), IPS[1], 1),
        build_capture(destination_table[2].clone(), networks[4].clone(), IPS[2], 2),
        build_capture(destination_table[3].clone(), networks[5].clone(), IPS[3], 3),
        build_capture(destination_table[4].clone(), networks[3].clone(), IPS[4], 4),
        build_capture(destination_table[5].clone(), networks[3].clone(), IPS[5], 5),
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
