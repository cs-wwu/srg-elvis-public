use std::time::Duration;

use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, Tcp,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network, Transport,
};

// TODO(hardint): There is a lot of redundant code with addresses and such. Consolidate.

/// Runs a basic simulation.
///
/// In this simulation, a machine sends a message to another machine over a
/// single network. The simulation ends when the message is received.
pub async fn tcp_with_reliable() {
    let network = Network::basic();
    let endpoint = Endpoint {
        address: [123, 45, 67, 89].into(),
        port: 0xbeef,
    };

    let sm_address = Ipv4Address::new([6, 0, 0, 0]);

    let ip_table: IpTable<Recipient> = [(sm_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message: Vec<_> = (0..20).map(|i| i as u8).collect();
    let message = Message::new(message);
    let machines = vec![
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            SendMessage::new(vec![message.clone()], endpoint)
                .transport(Transport::Tcp)
                .local_ip(sm_address),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1).transport(Transport::Tcp)
        ],
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
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
    #[tokio::test(flavor = "multi_thread")]
    async fn tcp_with_reliable() {
        super::tcp_with_reliable().await
    }
}
