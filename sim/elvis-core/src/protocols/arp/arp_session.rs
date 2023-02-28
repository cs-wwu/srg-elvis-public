use std::sync::Arc;

use crate::{
    control::{Key, Primitive},
    protocol::Context,
    protocols::{Ipv4, Pci},
    session::{QueryError, SendError, SharedSession},
    Message, Network, Session,
};

use super::{arp_parsing::ArpPacket, Arp};

pub struct ArpSession {
    /// The ARP protocol object that created this session.
    arp_protocol: Arc<Arp>,
    /// The PCI protocol to send messages through
    downstream: SharedSession,
}

impl ArpSession {
    pub fn new(arp_protocol: Arc<Arp>, downstream: SharedSession) -> Self {
        ArpSession {
            arp_protocol,
            downstream,
        }
    }

    /// Sends an ARP packet using the local ip and remote ip in the given context
    pub(super) fn send_arp_packet(
        &self,
        is_request: bool,
        mut context: Context,
    ) -> Result<(), SendError> {
        let local_mac = self
            .downstream
            .clone()
            .query(Pci::MAC_QUERY_KEY)
            .expect("unable to get MAC from Pci")
            .to_u64()
            .unwrap();

        let sender_ip =
            Ipv4::get_local_address(&context.control).expect("context does not have local ip");

        let target_ip =
            Ipv4::get_remote_address(&context.control).expect("context does not have remote ip");

        let target_mac = match is_request {
            true => {
                Network::set_destination(Network::BROADCAST_MAC, &mut context.control);
                Network::BROADCAST_MAC
            }
            false => Network::get_destination(&context.control)
                .expect("context does not have target MAC"),
        };

        let arp_request = ArpPacket {
            is_request,
            sender_ip,
            sender_mac: local_mac,
            target_ip,
            target_mac, // target mac is ignored for ARP requests
        };

        // Needed to make sure that another ARP layer receives this message
        Network::set_protocol(Arp::ID, &mut context.control);

        self.downstream
            .clone()
            .send(Message::new(arp_request.build()), context)
    }

    /// Sends an ARP request using the local ip and remote ip in the given context
    pub(super) fn send_arp_request(&self, context: Context) -> Result<(), SendError> {
        self.send_arp_packet(true, context)
    }

    /// Sends an ARP reply using the destination MAC, local ip, and remote ip in the given context
    pub(super) fn send_arp_reply(&self, context: Context) -> Result<(), SendError> {
        self.send_arp_packet(false, context)
    }
}

impl Session for ArpSession {
    fn send(self: Arc<Self>, message: Message, mut context: Context) -> Result<(), SendError> {
        assert_eq!(
            Network::get_protocol(&context.control),
            Ok(Ipv4::ID),
            "ArpSession::send should only be used to send IPv4 packets."
        );

        tokio::spawn(async move {
            let destination_mac = self.arp_protocol.clone().get_mac(&context).await;
            Network::set_destination(destination_mac, &mut context.control);

            // Because I cannot propogate the SendError from a thread, I have to panic!
            self.downstream
                .clone()
                .send(message, context)
                .expect("Got error after attempting to send ARP packet");
        });

        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}
