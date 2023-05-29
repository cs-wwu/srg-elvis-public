use crate::applications::{Capture, SendMessage};
use elvis_core::{
    machine::ProtocolMapBuilder,
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Recipient, Recipients},
        Endpoint, Pci, Tcp, UserProcess,
    },
    run_internet, Machine, Network, Transport,
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
    let ip_table: Recipients = [(endpoint.address, Recipient::with_mac(0, 1))]
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
                .process(),
        ],
        new_machine![
            Tcp::new(),
            Ipv4::new(ip_table),
            Pci::new([network.clone()]),
            Capture::new(endpoint, 1)
                .transport(Transport::Tcp)
                .process(),
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
    async fn tcp_with_reliable() {
        super::tcp_with_reliable().await
    }
}
