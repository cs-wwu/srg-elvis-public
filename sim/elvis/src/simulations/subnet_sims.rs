//! Simulations to test [`elvis_core::protocols::arp::subnetting`].

use std::sync::Arc;

use elvis_core::{
    machine::ProtocolMap,
    protocol::{DemuxError, StartError},
    protocols::{
        arp::{
            arp_parsing::ArpPacket,
            subnetting::{Ipv4Mask, SubnetInfo},
        },
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, Recipient, Recipients},
        Arp, Endpoint, Ipv4, Pci, Udp,
    },
    *,
};
use tokio::sync::{broadcast, Barrier};

use crate::applications::{OnReceive, SendMessage};

const PORT: u16 = 0xfefe;
// Mae is 30.40.50.12/24
const MAE: Endpoint = Endpoint::new(Ipv4Address::new([30, 40, 50, 12]), PORT);
// Jack is 30.40.50.13/24
const JACK: Endpoint = Endpoint::new(Ipv4Address::new([30, 40, 50, 13]), PORT);
// Default gateway is 30.40.50.17/24
const GATEWAY: Endpoint = Endpoint::new(Ipv4Address::new([30, 40, 50, 17]), PORT);
// Guy somewhere else is 30.40.90.12
const GUY_SOMEWHERE_ELSE: Endpoint = Endpoint::new(Ipv4Address::new([30, 40, 90, 12]), PORT);

// /24
const SUBNET_INFO: SubnetInfo = SubnetInfo::new(Ipv4Mask::from_bitcount(24), GATEWAY.address);

/// Returns a recipients table where all IPs go to tap slot 0
fn ip_table() -> Recipients {
    let recipient = Recipient::new(0, None);
    [
        MAE.address,
        GATEWAY.address,
        JACK.address,
        GUY_SOMEWHERE_ELSE.address,
    ]
    .into_iter()
    .map(|ip| (ip, recipient))
    .collect()
}

/// Mock gateway protocol
/// Reports when a message is received
struct MockGateway {
    send: broadcast::Sender<Message>,
}

#[async_trait::async_trait]
impl Protocol for MockGateway {
    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().expect("udp should be in map");
        udp.listen(self.id(), GATEWAY, protocols.clone())
            .expect("listen should not err");
        // listen on Guy Somewhere Else's address, so Ipv4 does not discard messages
        // intended for them
        // This is pretty janky and will change if the implementation of ipv4 changes...
        udp.listen(self.id(), GUY_SOMEWHERE_ELSE, protocols)
            .expect("listen should not err");
        initialize.wait().await;
        Ok(())
    }

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // Check to make sure we have the correct host and destination addresses
        println!("control of mock gateway: {:?}", control);
        let header = control
            .get::<Ipv4Header>()
            .expect("context should contain an address pair");
        assert_eq!(header.source, MAE.address);
        assert_eq!(header.destination, GUY_SOMEWHERE_ELSE.address);
        self.send.send(message).unwrap();
        Ok(())
    }
}

/// Creates Jack's machine
/// Jack sends messages to Mae
fn jack(network: &Arc<Network>) -> Machine {
    new_machine![
        SendMessage::new(vec![Message::new(b"hi mae this is jack")], MAE)
            .local_ip(JACK.address)
            .process(),
        Udp::new(),
        Ipv4::new(ip_table()),
        Arp::basic().preconfig_subnet(JACK.address, SUBNET_INFO),
        Pci::new([network.clone()]),
    ]
}

/// Creates Mae's machine, and a receiver for messages
/// Mae receives messages from Jack.
/// Mae sends a message to Guy Somewhere Else.
fn mae(network: &Arc<Network>) -> (Machine, broadcast::Receiver<Message>) {
    let (send, recv) = broadcast::channel(1);
    let message = vec![Message::new(b"hi guy somewhere else this is mae")];
    let maechine = new_machine![
        OnReceive::new(
            move |message, context| {
                println!("mae control: {:?}", context);
                send.send(message).unwrap();
            },
            MAE
        ),
        SendMessage::new(message, GUY_SOMEWHERE_ELSE)
            .local_ip(MAE.address)
            .process(),
        Udp::new(),
        Ipv4::new(ip_table()),
        Arp::basic().preconfig_subnet(MAE.address, SUBNET_INFO),
        Pci::new([network.clone()]),
    ];

    (maechine, recv)
}

/// Creates the default gateway machine
/// The default gateway (i.e. a router) would connect Mae to Guy Somewhere Else,
/// but for now it's just a mock gateway.
/// So it receives messages intended for Guy Somewhere Else.
fn gateway(
    network: &Arc<Network>,
) -> (
    Machine,
    broadcast::Receiver<Message>,
    broadcast::Receiver<ArpPacket>,
) {
    let (r_send, r_recv) = broadcast::channel(1);
    let (arp_send, arp_recv) = broadcast::channel(5);
    let arp_recv_hook = move |message: Message| {
        let packet = ArpPacket::from_bytes(message.iter()).expect("failed to parse ARP packet");
        arp_send.send(packet).unwrap();
    };
    let gateway = new_machine![
        MockGateway { send: r_send },
        Udp::new(),
        Ipv4::new(ip_table()),
        Arp::debug(|_, _| {}, arp_recv_hook),
        Pci::new([network.clone()]),
    ];

    (gateway, r_recv, arp_recv)
}

/// A function which tests subnetting using Arp.
/// Tests to make sure that machines will send messages to their default gateway when
/// trying to send a message outside their subnet.
pub async fn test_subnet() {
    let network = Network::basic();

    let (mae, mut mae_recv) = mae(&network);
    let (gateway, mut gateway_recv, mut gateway_arp_recv) = gateway(&network);

    // These 4 lines of code actually run the simulation!
    tokio::spawn(async move {
        let machines = [jack(&network), mae, gateway];
        run_internet(&machines).await;
    });

    // Wait for Mae to get a message from Jack
    assert_eq!(
        mae_recv.recv().await,
        Ok(Message::new(b"hi mae this is jack"))
    );
    println!("mae got the message from jack!");

    // make sure the gateway got a proper ARP request from Mae
    loop {
        let arp_packet = gateway_arp_recv.recv().await.unwrap();
        println!("arp request to gateway; {:?}", arp_packet);
        if arp_packet.sender_ip == MAE.address && arp_packet.target_ip == GATEWAY.address {
            println!("gateway got an arp request from mae!");
            break;
        }
    }

    // make sure the message intended for Guy Somewhere Else was sent to the gateway
    assert_eq!(
        gateway_recv.recv().await,
        Ok(Message::new(b"hi guy somewhere else this is mae"))
    );
    println!("gateway got the message from mae!");
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_subnet() {
        super::test_subnet().await;
    }
}
