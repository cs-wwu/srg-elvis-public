use std::time::Duration;

use crate::applications::{Capture, Terminal};
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network, run_internet,
};

/// In this simulation, 
pub async fn terminal_send() {
    let network = Network::basic();
    let message = Message::new("Hello!");
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };

    let local_address: Ipv4Address = [127, 0, 0, 1].into();

    let ip_table: IpTable<Recipient> = [(local_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            // SendMessage::new(vec![message.clone()], endpoint),
            Terminal::new(String::from("localhost:0")),
            Udp::new(),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1),
        ],
    ];

    let status = run_internet(&machines).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
        .into_inner()
        .protocol::<Capture>()
        .unwrap()
        .message();

    // print received??

    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn terminal_send() {
        super::terminal_send().await;
    }
}
