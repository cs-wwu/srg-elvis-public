use crate::applications::{
    web_server::{WebServer, WebServerType},
    SimpleWebClient,
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

/// Runs a simulation with <num_servers> WebServers who each have <num_clients / num_servers>
/// SimpleWebClients connected to them
pub async fn server_experiment() {
    let network = Network::basic();

    let num_clients: u32 = 100;
    let num_servers: u32 = 1; // Can only do 1 server right now since local host isn't implemented

    let mut client_ip_addresses: Vec<Ipv4Address> = vec![];
    let mut server_ip_addresses: Vec<Ipv4Address> = vec![];

    // Generate unique IP addresses for each server and client and add them to ip_map
    for i in 0..num_servers {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_server_ip_address = [100, 42, tens, ones].into(); // Ip addresses are arbitrary
        server_ip_addresses.push(this_server_ip_address);
    }

    // Generate unique IP addresses for each client
    for i in 0..num_clients {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_client_ip_address = [123, 45, tens, ones].into(); // Ip addresses are arbitrary
        client_ip_addresses.push(this_client_ip_address);
    }

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    // Create machines to run each server and client
    let mut machines = vec![];
    for i in 0..num_servers {
        machines.push(new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(server_ip_addresses[i as usize])),
            WebServer::new(WebServerType::Yahoo, Some(13)),
        ])
    }

    for i in 0..num_clients {
        let server_index = i % num_servers;
        machines.push(new_machine_arc![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Arp::new(),
            SocketAPI::new(Some(client_ip_addresses[i as usize])),
            SimpleWebClient::new(Endpoint::new(
                server_ip_addresses[server_index as usize],
                80
            )),
        ])
    }

    let status = run_internet_with_timeout(&machines, Duration::from_secs(5)).await;
    assert_eq!(status, ExitStatus::Exited);

    let mut machines_iter = machines.into_iter();
    for _i in 0..num_servers {
        // Get server machines out of the way
        let _server = machines_iter.next().unwrap();
    }

    // Iterate through each client machine to find the highest, lowest, and average number of pages recieved per client
    let mut high = 0;
    let mut low = std::u32::MAX;
    let mut total = 0;
    let mut no_pages_recvd_count = 0;
    for _i in 0..num_clients {
        let client = machines_iter.next().unwrap();
        let lock = &client
            .protocol::<SimpleWebClient>()
            .unwrap()
            .num_pages_recvd;
        let num_pages_recvd = *lock.read().unwrap();

        if num_pages_recvd > high {
            high = num_pages_recvd;
        }
        if num_pages_recvd < low {
            low = num_pages_recvd;
        }
        total += num_pages_recvd;

        if num_pages_recvd == 0 {
            no_pages_recvd_count += 1;
        }
    }
    let avg: f32 = total as f32 / num_clients as f32;
    println!(
        "Total: {}\nHigh: {}\nLow: {}\nAvg: {}",
        total, high, low, avg
    );
    assert_eq!(no_pages_recvd_count, 0);
}

#[cfg(test)]
mod tests {
    #[tokio::test(flavor = "multi_thread")]
    async fn server_experiment() {
        for _ in 0..5 {
            super::server_experiment().await;
        }
    }
}
