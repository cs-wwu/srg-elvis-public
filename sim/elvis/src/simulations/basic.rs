use std::time::Duration;

use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    network::{Mtu, NetworkBuilder},
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable,
};

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
///
/// # Arguments
///
/// * `message` - the message to be sent
///
/// * `mtu` - the MTU of the network
pub async fn basic(message: Message, mtu: Mtu) {
    // Create network with given mtu
    let network = NetworkBuilder::new().mtu(mtu).build();

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
            SendMessage::new(vec![message.clone()], endpoint),
            Udp::new(),
        ],
        new_machine![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1),
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);

    let received = machines
        .into_iter()
        .nth(1)
        .unwrap()
        .into_inner()
        .protocol::<Capture>()
        .unwrap()
        .message();

    assert_eq!(received, Some(message));
}

#[cfg(test)]
mod tests {
    use elvis_core::{network::Mtu, Message};

    #[tokio::test]
    async fn basic() {
        let message = Message::new("Hello");
        super::basic(message, Mtu::MAX).await
    }

    // A test that sets the network to a low MTU

    #[tokio::test]
    async fn basic_reassembly() {
        // build message made out of "bingus" repeated 8192 times
        let size = 8192;
        let mut message = String::with_capacity(size);
        let word = "bingus";
        while message.len() < size {
            message.push_str(word);
        }
        let message = Message::new(message);

        super::basic(message, 997).await;
    }
}
