use crate::applications::{
    web_server::{WebServer, WebServerType},
    UserBehavior,
};
use elvis_core::{
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Arp, Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};
use std::time::Duration;

pub async fn server_user() {
    let network = Network::basic();
    let server_ip_address: Ipv4Address = [100, 42, 0, 0].into();
    let client1_ip_address: Ipv4Address = [123, 45, 67, 90].into();
    let server_socket_address: Endpoint = Endpoint::new(server_ip_address, 80);

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    // need to loop this x amount of times
    let machines = vec![
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(server_ip_address)),
            WebServer::new(WebServerType::Yahoo, None),
        ],
        new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client1_ip_address)),
            UserBehavior::new(server_socket_address),
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);

    let mut machines_iter = machines.into_iter();
    let _server = machines_iter.next().unwrap();

    // Check that the user recieved at least 20 pages and images
    let client = machines_iter.next().unwrap();
    let lock = &client.protocol::<UserBehavior>().unwrap();
    let num_pages_recvd = *lock.num_pages_recvd.read().unwrap();
    let num_imgs_recvd = *lock.num_imgs_recvd.read().unwrap();

    assert!(num_pages_recvd > 20);
    assert!(num_imgs_recvd > 20);
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn server_user() {
        for _ in 0..5 {
            super::server_user().await;
        }
    }
}
