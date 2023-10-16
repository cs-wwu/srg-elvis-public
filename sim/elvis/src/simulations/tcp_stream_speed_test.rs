use crate::applications::{BareBonesClient, BareBonesServer};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, SocketAPI, Tcp,
    },
    run_internet_with_timeout, IpTable, Network,
};
use std::{collections::BTreeMap, time::Duration};

/// Runs a simulation with <num_servers> BareBonesServers who each have <num_clients / num_servers>
/// BareBonesClients connected to them. Designed to test ELVIS's speed.
pub async fn tcp_stream_speed_test() {
    let network = Network::basic();

    let num_clients: u32 = 2000;
    let num_servers: u32 = 1;

    let mut client_ip_addresses: Vec<Ipv4Address> = vec![];
    let mut server_ip_addresses: Vec<Ipv4Address> = vec![];

    let mut ip_map = BTreeMap::new();

    // Generate unique IP addresses for each server and client and add them to ip_map
    for i in 0..num_servers {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_server_ip_address = [100, 42, tens, ones].into(); // Ip addresses are arbitrary
        server_ip_addresses.push(this_server_ip_address);
        ip_map.insert(this_server_ip_address, Recipient::with_mac(0, i.into()));
    }

    for i in 0..num_clients {
        let tens: u8 = (i / 10).try_into().unwrap();
        let ones: u8 = (i % 10).try_into().unwrap();
        let this_client_ip_address = [123, 45, tens, ones].into(); // Ip addresses are arbitrary
        client_ip_addresses.push(this_client_ip_address);
        ip_map.insert(this_client_ip_address, Recipient::with_mac(0, 1));
    }

    // Convert ip_map to ip_table
    let ip_table: IpTable<Recipient> = ip_map.into_iter().collect();

    // Create machines to run each server and client
    let mut machines = vec![];
    for i in 0..num_servers {
        machines.push(new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(server_ip_addresses[i as usize])),
            BareBonesServer::new(Endpoint::new(server_ip_addresses[i as usize], 80)),
        ])
    }

    for i in 0..num_clients {
        let server_index = i % num_servers;
        machines.push(new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SocketAPI::new(Some(client_ip_addresses[i as usize])),
            BareBonesClient::new(Endpoint::new(
                server_ip_addresses[server_index as usize],
                80
            )),
        ])
    }

    run_internet_with_timeout(&machines, Duration::from_secs(10)).await;

    // Get server machines out of the way
    let mut machines_iter = machines.into_iter();
    for _i in 0..num_servers {
        let _server = machines_iter.next().unwrap();
    }

    // Iterate through each client machine to find the highest, lowest, and average number of pages recieved per client
    let mut high = 0;
    let mut low = std::u32::MAX;
    let mut total = 0;
    for _i in 0..num_clients {
        let client = machines_iter.next().unwrap();
        let lock = &client
            .into_inner()
            .protocol::<BareBonesClient>()
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

        assert!(num_pages_recvd > 0)
    }
    let avg: f32 = total as f32 / num_clients as f32;
    println!(
        "Total: {}\nHigh: {}\nLow: {}\nAvg: {}",
        total, high, low, avg
    );
}

#[cfg(test)]
mod tests {
    // #[tokio::test]
    async fn tcp_stream_speed_test() {
        super::tcp_stream_speed_test().await;
    }
}
