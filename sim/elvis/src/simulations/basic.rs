use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        udp::Udp,
        Endpoint, Pci, UserProcess,
    },
    run_internet, Network,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn basic() {
    let network = Network::basic();
    let message = Message::new("Hello!");
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint).process(),
            Udp::new(),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1).process(),
        ],
    ];

    run_internet(&machines).await;
    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
        .into_inner()
        .protocol::<UserProcess<Capture>>()
        .unwrap()
        .application()
        .message();
    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn basic() {
        super::basic().await
    }
}
