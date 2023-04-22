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

use tokio::sync::{watch, RwLock};

pub struct ArpSession {
    /// This session's local MAC address
    local_mac: Mac,
    /// This session's destination IP address
    dest_ip: Ipv4Address,
    /// This session's destination MAC address,
    pub(super) dest_mac: Arc<MacStatusGetter>,
    /// The PCI protocol to send messages through
    downstream: SharedSession,
}

impl ArpSession {
    /// Creates a new ArpSession.
    /// Panics if the downstream session is not a Pci session.
    pub fn new(dest_ip: Ipv4Address, dest_mac: Option<Mac>, downstream: SharedSession) -> Self {
        let local_mac = downstream
            .clone()
            .query(Pci::MAC_QUERY_KEY)
            .expect("unable to get MAC from Pci")
            .to_u64()
            .unwrap();

        ArpSession {
            local_mac,
            dest_ip,
            dest_mac: Arc::new(MacStatusGetter::new(dest_mac)),
            downstream,
        }
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
        // don't bother sending requests if the status is already set
        if self.dest_mac.get_status() != MacStatus::Waiting {
            return;
        }

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
                self.dest_mac.set_status(MacStatus::FailedToGet).await;
                return;
            }

            requests += 1;

            // Wait RESEND_DELAY seconds, or stop waiting early if receiver.changed() occured
            let timeout =
                tokio::time::timeout(Arp::RESEND_DELAY, self.dest_mac.wait_for_status()).await;

            // If the mac status has been set, break out
            if timeout.is_ok() {
                return;
            }

            // If we've sent enough requests, set the status to failed, and break out.
            if requests == Arp::RESEND_TRIES {
                self.dest_mac.set_status(MacStatus::FailedToGet).await;
                return;
            }
        }
    }

    pub(super) fn send_arp_reply(
        &self,
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
    /// (This will not attach any other data, so you can send messages with a different
    /// IP address through this session. Useful for building a router!)
    ///
    /// This will return SendError::Other if the ARP session could not resolve the destination MAC address.
    /// This will occur if there is no destination machine with the local IP address.
    ///
    /// This method may return Ok(()) even if a message failed to send!
    fn send(&self, message: Message, mut context: Context) -> Result<(), SendError> {
        // If this message already has a destination MAC, do nothing
        if Network::get_destination(&context.control).is_ok() {
            return self.downstream.clone().send(message, context);
        }

        // If we can get the status right away and it was set, just use that
        match self.dest_mac.get_status() {
            MacStatus::Set(mac) => {
                Network::set_destination(mac, &mut context.control);
                return self.downstream.send(message, context);
            }
            MacStatus::FailedToGet => {
                return Err(SendError::Other);
            }
            _ => {}
        }

        // Otherwise, we'll have some waiting to do
        let dest_mac = self.dest_mac.clone();
        let downstream = self.downstream.clone();
        tokio::spawn(async move {
            match dest_mac.wait_for_status().await {
                Some(mac) => {
                    Network::set_destination(mac, &mut context.control);
                    let send_result = downstream.send(message, context);
                    if let Err(e) = send_result {
                        tracing::error!("Failed to send package downstream: {}", e);
                    };
                }
                None => {
                    // I can't propogate a SendError from a task unfortunately
                    tracing::error!(
                        "Failed to get MAC address. Participants: {:?}",
                        context.control
                    );
                }
            }
        });
        // May be a lie
        Ok(())
    }

    fn query(&self, key: Key) -> Result<Primitive, QueryError> {
        self.downstream.clone().query(key)
    }
}

/// Used to set and get this session's MacStatus concurrently
pub(super) struct MacStatusGetter {
    /// send on this channel when the mac is updated
    notifier: watch::Sender<()>,
    data: RwLock<MacStatus>,
}

impl MacStatusGetter {
    pub fn new(mac: Option<Mac>) -> MacStatusGetter {
        let (notifier, _) = watch::channel(());
        let data = match mac {
            Some(mac) => MacStatus::Set(mac),
            None => MacStatus::Waiting,
        };
        let data = RwLock::new(data);
        MacStatusGetter { notifier, data }
    }

    pub async fn set_status(&self, status: MacStatus) {
        *self.data.write().await = status;
        // ignore error
        let _result = self.notifier.send(());
    }

    /// returns the current mac status
    pub fn get_status(&self) -> MacStatus {
        match self.data.try_read() {
            Ok(guard) => *guard,
            Err(_) => MacStatus::Waiting,
        }
    }

    /// Waits until the mac status is either set or failed to get
    /// returns Some(mac) if the mac resolved successfully
    /// returns None if the mac could not be resolved (because the other machine did not respond to ARP requests in time)
    pub async fn wait_for_status(&self) -> Option<Mac> {
        let mut receiver = self.notifier.subscribe();
        loop {
            match *self.data.read().await {
                MacStatus::Set(mac) => return Some(mac),
                MacStatus::FailedToGet => return None,
                MacStatus::Waiting => {
                    receiver
                        .changed()
                        .await
                        .expect("sender should not be dropped");
                }
            }
        }
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

impl Default for MacStatus {
    fn default() -> Self {
        MacStatus::Waiting
    }
}
