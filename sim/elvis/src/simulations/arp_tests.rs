use std::sync::mpsc::Sender;

use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4Address},
        Arp, Ipv4, Pci, Udp,
    },
    run_internet, Machine, Message, Network,
};
use tokio::sync::mpsc;

use crate::applications::{Capture, SendMessage};

pub async fn simple() {
    let network = Network::basic();

    // Set up IP table
    let sender_ip = Ipv4Address::from([123, 45, 67, 8]);
    let receiver_ip = Ipv4Address::from([67, 8, 9, 10]);
    let ip_table: IpToTapSlot = [(sender_ip, 0), (receiver_ip, 0)].into_iter().collect();

    // Machines
    let message = Message::new(b"hello");
    let machines = vec![
        // Receiver
        Machine::new([
            Capture::new(receiver_ip, 0xfefe, 1).shared() as SharedProtocol,
            Udp::new().shared(),
            Ipv4::new(ip_table.clone()).shared(),
            Arp::new().shared(),
            Pci::new([network.tap()]).shared(),
        ]),
        // Sender
        Machine::new([
            SendMessage::new(message, receiver_ip, 0xfefe).shared() as SharedProtocol,
            Udp::new().shared(),
            Ipv4::new(ip_table).shared(),
            Arp::new().shared(),
            Pci::new([network.tap()]).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn simple() {
        todo!("fix me");
        super::simple().await;
    }
}
