//! Several tests and simulations for the ARP protocol.
//! These tests are pretty janky. If you want to learn how to write a simulation in ELVIS,
//! look at some other simulation for examples.

use std::{sync::Arc, time::Duration};

use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4Address, Recipient},
        Arp, Endpoint, Endpoints, Ipv4, Pci, Udp,
    },
    run_internet, run_internet_with_timeout, ExitStatus, IpTable, Machine, Message, Network,
};

use crate::applications::{Capture, PingPong, SendMessage};

use tokio::sync::watch;

const SENDER_IP: Ipv4Address = Ipv4Address::new([123, 45, 67, 8]);
const SENDER_ENDPOINT: Endpoint = Endpoint::new(SENDER_IP, 0xfefe);
const RECEIVER_IP: Ipv4Address = Ipv4Address::new([67, 8, 9, 10]);
const RECEIVER_ENDPOINT: Endpoint = Endpoint::new(RECEIVER_IP, 0xfefe);

/// generates a Recipients to work with the simulations
fn ip_table() -> IpTable<Recipient> {
    let default_recipient: Recipient = Recipient::new(0, None);
    [
        // (RECEIVER_IP, default_recipient),
        (Ipv4Address::from([127, 0, 0, 1]), default_recipient),
    ]
    .into_iter()
    .collect()
}

/// generates a sender machine
fn sender_machine(network: &Arc<Network>, message: Message) -> Machine {
    new_machine!(
        SendMessage::new(vec![message], RECEIVER_ENDPOINT),
        // Used to set local IP
        Capture::new(SENDER_ENDPOINT, 1),
        Udp::new(),
        Ipv4::new(ip_table()),
        Arp::basic(),
        Pci::new([network.clone()]),
    )
}

/// generates a receiver machine
fn receiver_machine(network: &Arc<Network>) -> Machine {
    new_machine!(
        Capture::new(RECEIVER_ENDPOINT, 1),
        Udp::new(),
        Ipv4::new(Default::default()),
        Arp::basic(),
        Pci::new([network.clone()]),
    )
}

pub async fn simple() {
    let network = Network::basic();

    // Machines
    let message = Message::new(b"hello");
    let machines = vec![
        // Receiver
        receiver_machine(&network),
        // Sender
        sender_machine(&network, message),
    ];

    run_internet(&machines).await;
}

/// A simulation/test to make sure that the ARP protocol is actually
/// attaching a MAC address to packets, so that they are sent directly to
/// a machine instead of broadcasting.
pub async fn test_no_broadcast() {
    let network = Network::basic();
    let message = Message::new(b"super secret message that should not be broadcasted");

    let (send, mut recv) = tokio::sync::watch::channel(());
    recv.borrow_and_update();
    let evil_arp = Arp::debug(
        |_, _| {},
        move |_| {
            send.send_replace(());
        },
    );

    let machines = vec![
        // Receiver
        receiver_machine(&network),
        // Sender
        sender_machine(&network, message),
        // Evil guy who should not receive the message
        // TODO(sudobeans): when swappable protocols are supported, I would like to make sure that
        // the evil machine does not receive ipv4 messages.
        new_machine!(evil_arp, Ipv4::new(ip_table()), Pci::new([network.clone()]),),
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(2)).await;
    assert_eq!(status, ExitStatus::Exited);

    tokio::time::sleep(Duration::from_millis(2)).await;
    recv.changed()
        .await
        .expect("Evil machine should have received ARP request");
}

mod wait_to_send {
    use std::sync::Arc;

    use elvis_core::{
        machine::ProtocolMap,
        protocol::{DemuxError, StartError},
        protocols::{ipv4::ProtocolNumber, Ipv4},
        Control, Message, Protocol, Session,
    };

    use super::*;

    /// An application which doesn't set its local IP until 300 ms have passed.
    pub struct WaitToListen();

    #[async_trait::async_trait]
    impl Protocol for WaitToListen {
        async fn start(
            &self,
            _shutdown: elvis_core::Shutdown,
            initialize: Arc<tokio::sync::Barrier>,
            protocols: elvis_core::machine::ProtocolMap,
        ) -> Result<(), StartError> {
            initialize.wait().await;

            tokio::time::sleep(Duration::from_millis(300)).await;

            protocols
                .protocol::<Ipv4>()
                .unwrap()
                .listen(self.id(), RECEIVER_IP, protocols, ProtocolNumber::DEFAULT)
                .expect("listen should work");
            Ok(())
        }

        fn demux(
            &self,
            _: Message,
            _: Arc<dyn Session>,
            _: Control,
            _: ProtocolMap,
        ) -> Result<(), DemuxError> {
            Ok(())
        }
    }
}

/// A test to make sure that the ARP protocol resends ARP requests if the first one does not go through.
pub async fn test_resend() {
    let network = Network::basic();

    let make_arp = || {
        let (send, recv) = watch::channel(());
        let arp = Arp::debug(
            |_, _| {},
            move |_| {
                send.send_replace(());
            },
        );
        (arp, recv)
    };

    let (sender_arp, mut sender_arp_recv) = make_arp();
    let (receiver_arp, mut receiver_arp_recv) = make_arp();

    // Machines
    let message = Message::new(b"hello");
    let machines = vec![
        // Receiver
        new_machine!(
            wait_to_send::WaitToListen(),
            Ipv4::new(Default::default()),
            receiver_arp,
            Pci::new([network.clone()]),
        ),
        // Sender
        new_machine!(
            SendMessage::new(vec![message], RECEIVER_ENDPOINT),
            // Used to set local IP
            Capture::new(SENDER_ENDPOINT, 1),
            Udp::new(),
            Ipv4::new(ip_table()),
            sender_arp,
            Pci::new([network.clone()]),
        ),
    ];

    tokio::spawn(async move {
        let m = machines;
        run_internet(&m).await
    });

    // Make sure ARP request gets resent
    receiver_arp_recv
        .changed()
        .await
        .expect("receiver did not get 1st arp request");
    receiver_arp_recv
        .changed()
        .await
        .expect("receiver did not get 2nd arp request");
    // make sure ARP reply is sent
    sender_arp_recv
        .changed()
        .await
        .expect("sender did not receive ARP reply");
}

/// A version of the ping_pong simulation that uses Arp.
pub async fn ping_pong() {
    let network = Network::basic();

    let ping_table: IpTable<Recipient> = [(SENDER_ENDPOINT.address, Recipient::new(0, None))]
        .into_iter()
        .collect();

    let pong_table: IpTable<Recipient> = [(RECEIVER_ENDPOINT.address, Recipient::new(0, None))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine!(
            Udp::new(),
            Ipv4::new(ping_table),
            Arp::basic(),
            Pci::new([network.clone()]),
            PingPong::new(true, Endpoints::new(SENDER_ENDPOINT, RECEIVER_ENDPOINT)),
        ),
        new_machine!(
            Udp::new(),
            Ipv4::new(pong_table),
            Arp::basic(),
            Pci::new([network.clone()]),
            PingPong::new(false, Endpoints::new(RECEIVER_ENDPOINT, SENDER_ENDPOINT)),
        ),
    ];

    let status = run_internet_with_timeout(&machines, Duration::from_secs(3)).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn simple() {
        super::simple().await;
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_no_broadcast() {
        super::test_no_broadcast().await;
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_resend() {
        super::test_resend().await;
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn ping_pong() {
        super::ping_pong().await;
    }
}
