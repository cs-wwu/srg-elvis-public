use elvis_core::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, Recipient},
        Endpoint, Endpoints, Ipv4, Pci, Udp,
    },
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;

use crate::applications::ArpRouter;

use super::rip_parsing::RipPacket;

// number of seconds between each update
const UPDATE: u64 = 30;
pub struct RipRouter {
    local_ips: Vec<Ipv4Address>,
    inner: Arc<ArpRouter>,
}

impl RipRouter {
    pub fn new(
        // Maps subnet to a given router ip.
        // Setting route to none sets the destination ip to the destination
        // ip in the received packet so the router can send to a local network.
        ip_table: IpTable<(Option<Ipv4Address>, PciSlot)>,
        local_ips: Vec<Ipv4Address>,
    ) -> Self {
        RipRouter {
            local_ips: local_ips.clone(),
            inner: Arc::new(ArpRouter::new(ip_table, local_ips)),
        }
    }

    pub async fn run(
        inner: Arc<ArpRouter>,
        sessions: Vec<Arc<dyn Session>>,
        protocols: ProtocolMap,
    ) {
        // send initial full table request for each udp session
        let ftr = Message::new(RipPacket::new_full_table_request().build());

        for session in sessions.iter() {
            match session.send(ftr.clone(), protocols.clone()) {
                Ok(_) => {},
                Err(_) => {
                    return;
                }
            }
        }

        // every UPDATE seconds send a broadcast update to each of the
        // routers udp sessions to update the routers table
        loop {
            // send
            for session in sessions.iter() {

            }
            tokio::time::sleep(Duration::from_secs(UPDATE)).await;
        }
    }
}

#[async_trait::async_trait]
impl Protocol for RipRouter {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols
            .clone()
            .protocol::<Udp>()
            .expect("RipRouter requires Udp");

        let mut sessions = Vec::<Arc<dyn Session>>::new();

        let remote_endpoint = Endpoint::new(Ipv4Address::SUBNET, 520);

        for ip in self.local_ips.iter() {
            let local_endpoint = Endpoint::new(*ip, 520);
            let session = match udp
                .open_and_listen(
                    self.id(),
                    Endpoints {
                        local: local_endpoint,
                        remote: remote_endpoint.clone(),
                    },
                    protocols.clone(),
                )
                .await
            {
                Ok(out) => out,
                Err(_) => return Err(StartError::Other),
            };

            sessions.push(session);
        }

        self.inner
            .start(shutdown, initialized.clone(), protocols.clone())
            .await
            .expect("Failed to start sessions");

        initialized.wait().await;

        tokio::spawn(Self::run(self.inner.clone(), sessions, protocols));

        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        todo!()
    }
}
