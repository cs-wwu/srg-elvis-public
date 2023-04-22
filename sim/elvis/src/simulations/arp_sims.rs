//! Several tests and simulations for the ARP protocol.
//! These tests are pretty janky. If you want to learn how to write a simulation in ELVIS,
//! look at some other simulation for examples.

use std::{sync::Arc, time::Duration};

use elvis_core::{
    protocol::SharedProtocol,
    protocols::{
        ipv4::{Ipv4Address, Recipient, Recipients},
        Arp, Ipv4, Pci, SubWrap, Udp,
    },
    run_internet, Machine, Message, Network,
};

use crate::applications::{Capture, SendMessage};

const SENDER_IP: Ipv4Address = Ipv4Address::new([123, 45, 67, 8]);
const RECEIVER_IP: Ipv4Address = Ipv4Address::new([67, 8, 9, 10]);
const MISC_IP_1: Ipv4Address = Ipv4Address::new([69, 42, 0, 13]);
const MISC_IP_2: Ipv4Address = Ipv4Address::new([36, 21, 4, 3]);

/// generates a Recipients to work with the simulations
fn ip_table() -> Recipients {
    let default_recipient: Recipient = Recipient::new(0, None);
    [
        (SENDER_IP, default_recipient),
        (RECEIVER_IP, default_recipient),
        (MISC_IP_1, default_recipient),
        (MISC_IP_2, default_recipient),
    ]
    .into_iter()
    .collect()
}

/// generates a sender machine
fn sender_machine(network: &Arc<Network>, message: Message) -> Machine {
    Machine::new([
        SendMessage::new(vec![message], RECEIVER_IP, 0xfefe).shared() as SharedProtocol,
        // Used to set local IP
        Capture::new(SENDER_IP, 0x0000, 1).shared(),
        Udp::new().shared(),
        Ipv4::new(ip_table()).shared(),
        Arp::new().shared(),
        Pci::new([network.clone()]).shared(),
    ])
}

/// generates a receiver machine
fn receiver_machine(network: &Arc<Network>) -> Machine {
    Machine::new([
        Capture::new(RECEIVER_IP, 0xfefe, 1).shared() as SharedProtocol,
        Udp::new().shared(),
        Ipv4::new(ip_table()).shared(),
        Arp::new().shared(),
        Pci::new([network.clone()]).shared(),
    ])
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

    run_internet(machines, vec![network]).await;
}

/// A simulation/test to make sure that the ARP protocol is actually
/// attaching a MAC address to packets, so that they are sent directly to
/// a machine instead of broadcasting.
pub async fn test_no_broadcast() {
    let network = Network::basic();
    let message = Message::new(b"super secret message that should not be broadcasted");

    let mut evil_arp = SubWrap::new(Arp::new());
    let mut evil_ipv4 = SubWrap::new(Ipv4::new(ip_table()));

    let mut evil_arp_recv = evil_arp.subscribe_demux();
    let mut evil_ipv4_recv = evil_ipv4.subscribe_demux();

    let machines = vec![
        // Receiver
        receiver_machine(&network),
        // Sender
        sender_machine(&network, message),
        // Evil guy who should not receive the message
        Machine::new([
            evil_arp.shared() as SharedProtocol,
            evil_ipv4.shared(),
            Pci::new([network.clone()]).shared(),
        ]),
    ];

    run_internet(machines, vec![network]).await;
    tokio::time::sleep(Duration::from_millis(2)).await;
    evil_arp_recv
        .recv()
        .await
        .expect("Evil machine should have received ARP request");
    if let Ok((_message, context)) = evil_ipv4_recv.try_recv() {
        let control = context.control;
        panic!("Evil machine should not have received IPv4 message. Control: {control:?}");
    }
}

mod wait_to_send {
    use std::sync::Arc;

    use elvis_core::{
        protocol::Context,
        protocols::{
            user_process::{Application, ApplicationError},
            Ipv4, UserProcess,
        },
        Control, Id, Message,
    };

    use super::*;

    /// An application which doesn't set its local IP until 300 ms have passed.
    pub struct WaitToListen();

    impl Application for WaitToListen {
        const ID: Id = Id::from_string("wait to send");

        fn start(
            &self,
            _shutdown: elvis_core::Shutdown,
            initialize: Arc<tokio::sync::Barrier>,
            protocols: elvis_core::ProtocolMap,
        ) -> Result<(), ApplicationError> {
            tokio::spawn(async move {
                initialize.wait().await;

                tokio::time::sleep(Duration::from_millis(300)).await;

                let mut participants = Control::new();
                Ipv4::set_local_address(RECEIVER_IP, &mut participants);
                Ipv4::set_remote_address(SENDER_IP, &mut participants);
                protocols
                    .protocol(Ipv4::ID)
                    .unwrap()
                    .listen(Self::ID, participants, protocols)
                    .expect("listen should work");
            });
            Ok(())
        }

        fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    impl WaitToListen {
        pub fn shared(self) -> Arc<UserProcess<Self>> {
            Arc::new(UserProcess::new(self))
        }
    }
}

/// A test to make sure that the ARP protocol resends ARP requests if the first one does not go through.
pub async fn test_resend() {
    let network = Network::basic();

    let make_arp = || {
        let mut arp = SubWrap::new(Arp::new());
        let recv = arp.subscribe_demux();
        (arp, recv)
    };

    let (sender_arp, mut sender_arp_recv) = make_arp();
    let (receiver_arp, mut receiver_arp_recv) = make_arp();

    // Machines
    let message = Message::new(b"hello");
    let machines = vec![
        // Receiver
        Machine::new([
            wait_to_send::WaitToListen().shared() as SharedProtocol,
            Ipv4::new(ip_table()).shared(),
            receiver_arp.shared(),
            Pci::new([network.clone()]).shared(),
        ]),
        // Sender
        Machine::new([
            SendMessage::new(vec![message], RECEIVER_IP, 0xfefe).shared() as SharedProtocol,
            // Used to set local IP
            Capture::new(SENDER_IP, 0x0000, 1).shared(),
            Udp::new().shared(),
            Ipv4::new(ip_table()).shared(),
            sender_arp.shared(),
            Pci::new([network.clone()]).shared(),
        ]),
    ];

    tokio::spawn(run_internet(machines, vec![network]));

    // Make sure ARP request gets resent
    receiver_arp_recv
        .recv()
        .await
        .expect("receiver did not get 1st arp request");
    receiver_arp_recv
        .recv()
        .await
        .expect("receiver did not get 2nd arp request");
    // make sure ARP reply is sent
    sender_arp_recv
        .recv()
        .await
        .expect("sender did not receive ARP reply");
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
}
