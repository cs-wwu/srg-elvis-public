use crate::applications::{rip::rip_router::RipRouter, ArpRouter, Capture, SendMessage};
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

const ROUTER2_IPS: [Ipv4Address; 3] = [
    Ipv4Address::new([123, 45, 66, 92]),
    Ipv4Address::new([123, 45, 66, 93]),
    Ipv4Address::new([123, 45, 66, 94]),
];

pub fn build_ip_table(addresses: &[Ipv4Address]) -> IpTable<Recipient> {
    let mut router_table = IpTable::<Recipient>::new();
    let mut slot = 0;
    for address in addresses.iter() {
        router_table.add_direct(*address, Recipient::new(slot, None));
        slot += 1;
    }
    router_table
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
pub async fn rip_router(destination: Ipv4Address) -> ExitStatus {
    let router_table_1: IpTable<(Option<Ipv4Address>, PciSlot)> = [
        (IPS[0], (None, 0)),
        (IPS[1], (None, 1)),
        (IPS[4], (None, 3)),
        (IPS[5], (None, 3)),
    ]
    .into_iter()
    .collect();

    let router_table_2: IpTable<(Option<Ipv4Address>, PciSlot)> =
        [(IPS[2], (None, 1)), (IPS[3], (None, 2))]
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
    ).delay(Duration::from_secs(5));

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
            Udp::new(),
            ArpRouter::new(router_table_1, ROUTER1_IPS.to_vec()),
            RipRouter::new(ROUTER1_IPS.to_vec())
        ],
        new_machine![
            Pci::new([
                networks[2].clone(),
                networks[4].clone(),
                networks[5].clone(),
            ]),
            Ipv4::new(ip_table_2),
            Arp::basic(),
            Udp::new(),
            ArpRouter::new(router_table_2, ROUTER2_IPS.to_vec()),
            RipRouter::new(ROUTER2_IPS.to_vec())
        ],
        // Destinations
        build_capture(networks[1].clone(), IPS[1], 1),
        build_capture(networks[4].clone(), IPS[2], 2),
        build_capture(networks[5].clone(), IPS[3], 3),
        build_capture(networks[3].clone(), IPS[4], 4),
        build_capture(networks[3].clone(), IPS[5], 5),
    ];

    run_internet_with_timeout(&machines, Duration::from_secs(20)).await

}

/* routes packet from source 0 to one of the given destinations 1,2,3,4,5

    0 -(0)- R1 -(1)- R2 -(2)- R3 -

*/
#[allow(dead_code)]
// pub async fn rip_router2(destination: Ipv4Address) -> ExitStatus {
//     let router_table_1: IpTable<(Option<Ipv4Address>, PciSlot)> = [
//         (IPS[0], (None, 0)),
//         (IPS[1], (None, 1)),
//         (IPS[4], (None, 3)),
//         (IPS[5], (None, 3)),
//     ]
//     .into_iter()
//     .collect();

//     let router_table_2: IpTable<(Option<Ipv4Address>, PciSlot)> =
//         [(IPS[2], (None, 1)), (IPS[3], (None, 2))]
//             .into_iter()
//             .collect();

//     let networks: Vec<_> = (0..6).map(|_| Network::basic()).collect();
//     let ip_table_1 = build_ip_table(&ROUTER1_IPS);
//     let ip_table_2 = build_ip_table(&ROUTER2_IPS);

//     let send_message = SendMessage::new(
//         vec![Message::new(b"Hello World!")],
//         Endpoint {
//             address: destination,
//             port: 0xbeef,
//         },
//     ).delay(Duration::from_secs(5));

//     let machines = vec![
//         // send message
//         new_machine![
//             Udp::new(),
//             Ipv4::new([(IPS[0], Recipient::new(0, None))].into_iter().collect(),),
//             Pci::new([networks[0].clone()]),
//             send_message.local_ip(IPS[0]),
//             Arp::basic().preconfig_subnet(
//                 IPS[0],
//                 SubnetInfo {
//                     mask: Ipv4Mask::from_bitcount(32),
//                     default_gateway: ROUTER1_IPS[0]
//                 }
//             ),
//         ],
//         // Routers
//         new_machine![
//             Pci::new([
//                 networks[0].clone(),
//                 networks[1].clone(),
//                 networks[2].clone(),
//                 networks[3].clone()
//             ]),
//             Ipv4::new(ip_table_1),
//             Arp::basic(),
//             Udp::new(),
//             ArpRouter::new(router_table_1, ROUTER1_IPS.to_vec()),
//             RipRouter::new(ROUTER1_IPS.to_vec())
//         ],
//         new_machine![
//             Pci::new([
//                 networks[2].clone(),
//                 networks[4].clone(),
//                 networks[5].clone(),
//             ]),
//             Ipv4::new(ip_table_2),
//             Arp::basic(),
//             Udp::new(),
//             ArpRouter::new(router_table_2, ROUTER2_IPS.to_vec()),
//             RipRouter::new(ROUTER2_IPS.to_vec())
//         ],
//         // Destinations
//         build_capture(networks[1].clone(), IPS[1], 1),
//         build_capture(networks[4].clone(), IPS[2], 2),
//         build_capture(networks[5].clone(), IPS[3], 3),
//         build_capture(networks[3].clone(), IPS[4], 4),
//         build_capture(networks[3].clone(), IPS[5], 5),
//     ];

//     run_internet_with_timeout(&machines, Duration::from_secs(20)).await

// }

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn rip_router() {
        let test1 = super::rip_router(super::IPS[1]);
        let test2 = super::rip_router(super::IPS[2]);
        let test3 = super::rip_router(super::IPS[3]);
        let test4 = super::rip_router(super::IPS[4]);

        assert_eq!(test1.await, super::ExitStatus::Status(1));
        assert_eq!(test2.await, super::ExitStatus::Status(2));
        assert_eq!(test3.await, super::ExitStatus::Status(3));
        assert_eq!(test4.await, super::ExitStatus::Status(4));
    }
}
