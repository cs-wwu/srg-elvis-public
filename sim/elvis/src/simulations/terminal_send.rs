use crate::applications::{Capture, Terminal};
use elvis_core::{
    message::Message,
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    ExitStatus, IpTable, Network, run_internet,
};

/// In this simulation, 
pub async fn terminal_send() {
    let network = Network::basic();
    let message = Message::new("Hello!");
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef, // 48879
    };
    let local = Endpoint {
        address: [123, 44, 66, 88].into(),
        port: 0xfeed,
    };

    let ip_table: IpTable<Recipient> = [(local.address, Recipient::with_mac(0, 1)), (endpoint.address, Recipient::with_mac(0, 0))]
        .into_iter()
        .collect();
    
    let machines = vec![
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            // SendMessage::new(vec![message.clone()], endpoint),
            Terminal::new(local, String::from("localhost:0")),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1),
        ],
    ];

    let status = run_internet(&machines, None).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
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
