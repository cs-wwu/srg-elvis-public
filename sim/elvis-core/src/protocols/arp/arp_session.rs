use std::sync::Arc;

use crate::{
    control::{Key, Primitive},
    network::Mac,
    protocol::Context,
    protocols::{ipv4::Ipv4Address, Ipv4, Pci},
    session::{QueryError, SendError, SharedSession},
    Message, Network, ProtocolMap, Session,
};

use super::{arp_parsing::ArpPacket, Arp};

use tokio::sync::{broadcast, RwLock, TryLockError};

pub struct ArpSession {
    /// This session's local MAC address
    local_mac: Mac,
    /// This session's destination IP address
    dest_ip: Ipv4Address,
    /// This session's destination MAC address
    dest_mac: RwLock<MacStatus>,
    /// This sender should be sent on after the dest_mac is updated.
    sender: broadcast::Sender<()>,
    /// The PCI protocol to send messages through
    downstream: SharedSession,
}

impl ArpSession {
    /// Creates a new ArpSession.
    /// Panics if the downstream session is not a Pci session.
    pub fn new(dest_ip: Ipv4Address, dest_mac: Option<Mac>, downstream: SharedSession) -> Self {
        let (sender, _) = broadcast::channel(1);
        let dest_mac = match dest_mac {
            Some(mac) => RwLock::new(MacStatus::Set(mac)),
            None => RwLock::new(MacStatus::Waiting),
        };
        let local_mac = downstream.clone().query(Pci::MAC_QUERY_KEY)
            .expect("unable to get MAC from Pci")
            .to_u64()
            .unwrap();

        ArpSession {
            local_mac,
            dest_ip,
            dest_mac,
            sender,
            downstream,
        }
    }

    /// Gets the status of this ARP session's destination MAC address.
    /// Will return error if could not acquire a read lock immediately.
    pub(super) fn try_get_status(self: Arc<Self>) -> Result<MacStatus, TryLockError> {
        match self.dest_mac.try_read() {
            Ok(guard) => Ok(*guard),
            Err(e) => Err(e),
        }
    }

    /// Gets the status of this ARP session's destination MAC address.
    pub(super) async fn get_status(self: Arc<Self>) -> MacStatus {
        *self.dest_mac.read().await
    }

    /// Sets the destination MAC address status of this ARP session,
    /// and notifies any threads waiting for the destination MAC address to be set.
    pub(super) async fn set_status(self: Arc<Self>, new_status: MacStatus) {
        let mut guard = self.dest_mac.write().await;
        *guard = new_status;
        drop(guard);
        let _ = self.sender.clone().send(());
    }

    /// Repeatedly sends ARP requests.
    ///
    /// Repeats until one of the following occurs:
    ///
    /// * We have already sent a number of requests equal to[`Arp::RESEND_TRIES`],
    ///
    /// * This session's MacStatus is [`MacStatus::Set`] or [`MacStatus::FailedToGet`].
    pub(super) async fn send_arp_requests(
        self: Arc<Self>,
        sender_ip: Ipv4Address,
        protocols: ProtocolMap,
    ) {
        let mut update_receiver = self.sender.subscribe();

        // Return if the MAC is already set or failed to get
        match self.clone().get_status().await {
            MacStatus::FailedToGet | MacStatus::Set(_) => return,
            MacStatus::Waiting => (),
        };

        let arp_request = ArpPacket {
            is_request: true,
            sender_ip,
            sender_mac: self.local_mac,
            target_ip: self.dest_ip,
            target_mac: Network::BROADCAST_MAC, // target mac is ignored for ARP requests
        };
        let message = Message::new(arp_request.build());

        let mut context = Context::new(protocols);

        // Needed to make sure that another ARP layer receives this message
        Network::set_protocol(Arp::ID, &mut context.control);

        Network::set_sender(self.local_mac, &mut context.control);
        Network::set_destination(Network::BROADCAST_MAC, &mut context.control);
        Ipv4::set_local_address(sender_ip, &mut context.control);
        Ipv4::set_remote_address(self.dest_ip, &mut context.control);

        // Repeatedly send ARP requests
        let mut requests = 0;
        loop {
            let send_result = self
                .downstream
                .clone()
                .send(message.clone(), context.clone());

            if let Err(e) = send_result {
                tracing::error!("failed to send ARP request: {:?}", e);
                self.set_status(MacStatus::FailedToGet).await;
                return;
            }

            requests += 1;

            // Wait RESEND_DELAY seconds, or stop waiting early if receiver.changed() occured
            let timeout = tokio::time::timeout(Arp::RESEND_DELAY, update_receiver.recv()).await;

            // If we've sent 10 requests, set the status to failed, and break out.
            if requests == 10 {
                self.set_status(MacStatus::FailedToGet).await;
                return;
            }

            // If receiver.changed(), break out
            if timeout.is_ok() {
                return;
            }
        }
    }

    pub(super) fn send_arp_reply(
        self: Arc<Self>,
        local_ip: Ipv4Address,
        remote_mac: Mac,
        protocols: ProtocolMap,
    ) -> Result<(), SendError> {
        let arp_reply = ArpPacket {
            is_request: false,
            sender_ip: local_ip,
            sender_mac: self.local_mac,
            target_mac: remote_mac,
            target_ip: self.dest_ip,
        };
        let message = Message::new(arp_reply.build());

        let mut context = Context::new(protocols);

        // Needed to make sure that another ARP layer receives this message
        Network::set_protocol(Arp::ID, &mut context.control);

        Network::set_sender(self.local_mac, &mut context.control);
        Network::set_destination(remote_mac, &mut context.control);
        Ipv4::set_local_address(local_ip, &mut context.control);
        Ipv4::set_remote_address(self.dest_ip, &mut context.control);

        self.downstream.clone().send(message, context)
    }
}

impl Session for ArpSession {
    /// Sends a message from the upstream session down to the PCI session,
    /// attaching a destination MAC address if one is not already attached.
    /// 
    /// This will return SendError::Other if the ARP session could not resolve the destination MAC address.
    /// This will occur if there is no destination machine with the local IP address.
    /// 
    /// This method may return Ok(()) even if a message failed to send!
    fn send(self: Arc<Self>, message: Message, mut context: Context) -> Result<(), SendError> {
        // If we can get the status right away and it was set, just use that
        match self.clone().try_get_status() {
            Ok(MacStatus::Set(mac)) => {
                Network::set_destination(mac, &mut context.control);
                return self.downstream.clone().send(message, context);
            },
            Ok(MacStatus::FailedToGet) => {
                return Err(SendError::Other);
            }
            _ => {}
        }

        // Otherwise, we'll have some waiting to do
        tokio::spawn(async move {
            let mut receiver = self.sender.subscribe();
            loop {
                match self.clone().get_status().await {
                    MacStatus::Set(mac) => {
                        Network::set_destination(mac, &mut context.control);
                        let send_result = self.downstream.clone().send(message, context);
                        if let Err(e) = send_result {
                            tracing::error!("Failed to send package downstream: {}", e);
                        };
                        break;
                    }
                    MacStatus::Waiting => {
                        let _ = receiver.recv().await; // don't care if it's Ok or Err
                    }
                    MacStatus::FailedToGet => {
                        // I can't propogate a SendError from a task unfortunately
                        tracing::error!(
                            "Failed to get MAC address. Participants: {:?}",
                            context.control
                        );
                        break;
                    }
                }
            }
        });
        // May be a lie
        Ok(())
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}

/// Used for an ArpSession's dest_mac.
/// Indicates whether the session's MAC has been set, or is waiting to be set.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MacStatus {
    /// Contains the MAC address.
    Set(Mac),
    /// Indicates this session is waiting for a MAC address.
    Waiting,
    /// Indicates that this session failed to get a MAC address.
    FailedToGet,
}
