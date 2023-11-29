use crate::applications::Terminal;
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    ExitStatus, IpTable, Network, run_internet,
};

/// In this simulation, 
pub async fn terminal_receive() {
    let network = Network::basic();
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef, // 48879
    };
    let local = Endpoint {
        address: [123, 44, 66, 88].into(),
        port: 0xfeed, // 65261
    };

    let ip_table: IpTable<Recipient> = [(local.address, Recipient::with_mac(0, 1)), (endpoint.address, Recipient::with_mac(0, 0))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Terminal::new(local, String::from("localhost:0")),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Terminal::new(endpoint, String::from("localhost:0")),
        ],
    ];

    let status = run_internet(&machines).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn terminal_receive() {
        super::terminal_receive().await;
    }
}
