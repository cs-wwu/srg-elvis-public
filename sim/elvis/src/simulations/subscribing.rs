//! This module contains several test simulations and examples of how to use the [`SubWrap`] protocol.

use std::collections::HashSet;

use elvis_core::{
    network::{Baud, NetworkBuilder, Throughput},
    protocol::{Context, SharedProtocol},
    protocols::{
        ipv4::{Ipv4Address, Recipient, Recipients},
        Ipv4, Pci, SubWrap, Udp, UserProcess,
    },
    run_internet, Machine, Message, Network, Protocol,
};
use tokio::sync::mpsc;

use crate::applications::{Capture, PingPong, SendMessage};

// Helper function to make it easy to subscribe_demux to a protocol
fn sub_demux(
    protocol: impl Protocol + Sync + Send + 'static,
) -> (SubWrap, mpsc::UnboundedReceiver<(Message, Context)>) {
    let mut sub = SubWrap::new(protocol);
    let recv = sub.subscribe_demux();
    (sub, recv)
}

// Helper function to make it easy to subscribe_send to a protocol
fn sub_send(
    protocol: impl Protocol + Sync + Send + 'static,
) -> (SubWrap, mpsc::UnboundedReceiver<(Message, Context)>) {
    let mut sub = SubWrap::new(protocol);
    let recv = sub.subscribe_send();
    (sub, recv)
}

fn message_contains(message: &Message, str: &str) -> bool {
    String::from_iter(message.iter().map(char::from)).contains(str)
}

/// A test simulation for the [`SubWrap`].
/// Based on the [`basic.rs`](crate::simulations::basic) simulation.
pub async fn basic_with_1_subscribe() {
    let network = Network::basic();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new(b"Hello!");

    // Subscribe to machine #1's Pci protocol
    let (pci, mut pci_recv) = sub_send(Pci::new([network.tap()]));

    let machines = vec![
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            pci.shared(),
            SendMessage::new(vec![message], capture_ip_address, 0xfefe).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            Capture::new(capture_ip_address, 0xfefe, 1).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;

    // See the message sent through PCI
    let message = pci_recv.recv().await.unwrap().0;
    println!(
        "message sent by pci: {:?}",
        String::from_iter(message.iter().map(char::from))
    );
    assert!(message_contains(&message, "Hello!"));
}

/// A version of [`basic_with_1_subscribe()`] with more rigorous testing of `SubWrap`.
pub async fn basic_with_lots_of_subscribe() {
    let network = Network::basic();
    let capture_ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let ip_table: Recipients = [(capture_ip_address, Recipient::with_mac(0, 1))]
        .into_iter()
        .collect();

    let message = Message::new(b"Hello!");

    // Subscribe to machine #1's Udp and Pci protocols
    let (udp, mut udp_recv) = sub_send(Udp::new());
    let (pci, mut pci_recv) = sub_send(Pci::new([network.tap()]));

    // Subscribe to machine #2's Ipv4 and capture protocols
    let (ipv4, mut ipv4_recv) = sub_demux(Ipv4::new(ip_table.clone()));
    let capture = UserProcess::new(Capture::new(capture_ip_address, 0xfefe, 1));
    let (capture, mut capture_recv) = sub_demux(capture);

    let machines = vec![
        Machine::new([
            udp.shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            pci.shared(),
            SendMessage::new(vec![message], capture_ip_address, 0xfefe).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            ipv4.shared(),
            Pci::new([network.tap()]).shared(),
            capture.shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;

    // See what messages were sent over the network
    assert_eq!(udp_recv.recv().await.unwrap().0, Message::new(b"Hello!"));

    let (pci_message, pci_context) = pci_recv.recv().await.unwrap();
    assert!(message_contains(&pci_message, "Hello!"));
    println!("PCI context: {:?}", pci_context.control);
    assert_eq!(Pci::get_pci_slot(&pci_context.control), Ok(0));

    let (ipv4_message, ipv4_context) = ipv4_recv.recv().await.unwrap();
    assert!(message_contains(&ipv4_message, "Hello!"));
    assert_eq!(Network::get_sender(&ipv4_context.control), Ok(0));

    assert_eq!(
        capture_recv.recv().await.unwrap().0,
        Message::new(b"Hello!")
    );
}

/// A version of the [`ping_pong.rs`](crate::simulations::ping_pong) simulation which prints out each ping and pong.
pub async fn print_ping_pong() {
    const IP_ADDRESS_1: Ipv4Address = Ipv4Address::new([123, 45, 67, 89]);
    const IP_ADDRESS_2: Ipv4Address = Ipv4Address::new([123, 45, 67, 90]);

    let network = NetworkBuilder::new()
        .throughput(Throughput::constant(Baud::bytes_per_second(65536)))
        .build();

    let ip_table: Recipients = [
        (IP_ADDRESS_1, Recipient::with_mac(0, 0)),
        (IP_ADDRESS_2, Recipient::with_mac(0, 1)),
    ]
    .into_iter()
    .collect();

    let udp_1 = Udp::new();
    let (udp_1, mut udp_1_recv) = sub_send(udp_1);

    let udp_2 = Udp::new();
    let (udp_2, mut udp_2_recv) = sub_send(udp_2);

    let machines = vec![
        // Machine 1
        Machine::new([
            udp_1.shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            PingPong::new(true, IP_ADDRESS_1, IP_ADDRESS_2, 0xfefe, 0xface).shared(),
        ]),
        // Machine 2
        Machine::new([
            udp_2.shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            PingPong::new(false, IP_ADDRESS_2, IP_ADDRESS_1, 0xface, 0xfefe).shared(),
        ]),
    ];

    tokio::spawn(run_internet(machines, vec![network]));

    // Print out each number sent by a machine
    let mut nums_set = HashSet::new();
    nums_set.extend(1..255);
    loop {
        let (message, direction) = tokio::select! {
            message = udp_1_recv.recv() => (message.unwrap().0, "->"),
            message = udp_2_recv.recv() => (message.unwrap().0, "<-"),
        };

        let num = message.iter().next().unwrap();
        nums_set.remove(&num);
        println!("[1] {direction} {num} {direction} [2]");

        if num == 1 {
            break;
        }
    }

    // make sure every number between 1 and 255 was seen
    assert!(nums_set.is_empty());
}

/// A test to make sure that:
/// * calling subscribe() multiple times doesn't break anything
/// * dropping the channel doesn't break anything
#[allow(dead_code)]
async fn subscribe_multiple_times() {
    let network = Network::basic();
    let ip_addr1: Ipv4Address = [123, 45, 67, 89].into();
    let ip_addr2: Ipv4Address = [42, 0, 62, 1].into();
    let ip_table: Recipients = [
        (ip_addr1, Recipient::with_mac(0, 1)),
        (ip_addr2, Recipient::with_mac(0, 0)),
    ]
    .into_iter()
    .collect();

    // Subscribe to machine #1's Pci protocol
    let mut udp = SubWrap::new(Udp::new());
    let mut demux_chan1 = udp.subscribe_demux();
    let mut demux_chan2 = udp.subscribe_demux();
    let mut send_chan1 = udp.subscribe_send();

    // make sure the sim still runs if you drop the subscriber
    let mut pci = SubWrap::new(Pci::new([network.tap()]));
    let dead_stream = pci.subscribe_demux();
    drop(dead_stream);

    let machines = vec![
        Machine::new([
            udp.shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            Pci::new([network.tap()]).shared(),
            PingPong::new(true, ip_addr1, ip_addr2, 0xfefe, 0xfefe).shared(),
        ]),
        Machine::new([
            Udp::new().shared() as SharedProtocol,
            Ipv4::new(ip_table.clone()).shared(),
            pci.shared(),
            PingPong::new(false, ip_addr2, ip_addr1, 0xfefe, 0xfefe).shared(),
        ]),
    ];

    // Assert the correct messages went through the machine
    tokio::join!(run_internet(machines, vec![network]), async move {
        assert_eq!(send_chan1.recv().await.unwrap().0, Message::from([255]));
        assert_eq!(demux_chan1.recv().await.unwrap().0.iter().last(), Some(254));
        assert_eq!(demux_chan2.recv().await.unwrap().0.iter().last(), Some(254));
    });
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn basic_with_1_subscribe() {
        super::basic_with_1_subscribe().await
    }

    #[tokio::test]
    async fn basic_with_lots_of_subscribe() {
        super::basic_with_lots_of_subscribe().await
    }

    #[tokio::test]
    async fn print_ping_pong() {
        super::print_ping_pong().await
    }

    /// A test to make sure that:
    /// * calling subscribe() multiple times doesn't break anything
    /// * dropping the channel doesn't break anything
    #[tokio::test]
    async fn subscribe_multiple_times() {
        super::subscribe_multiple_times().await;
    }
}
