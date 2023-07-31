use elvis_core::{
    machine::{PciSlot, ProtocolMap},
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{ipv4::{Ipv4Address, ipv4_parsing::Ipv4Header}, Endpoint, Endpoints, Udp, pci::DemuxInfo},
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;

use crate::applications::ArpRouter;

use super::rip_parsing::{Operation, RipPacket};

// number of seconds between each update
const UPDATE: u64 = 1;
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

        // possible bug: if a router does not receive this request then there is a chance that
        // its route is never discovered.
        for session in sessions.iter() {
            match session.send(ftr.clone(), protocols.clone()) {
                Ok(_) => {}
                Err(_) => {
                    return;
                }
            }
        }

        // every UPDATE seconds send a broadcast update to each of the
        // routers udp sessions to update the routers table
        loop {
            tokio::time::sleep(Duration::from_secs(UPDATE)).await;

            // obtain all routes not directly connected to the router
            let packets = inner.generate_request();

            // broadcast request to all adjacent routes
            for packet in packets.iter() {
                let message = Message::new(packet.build());
                for session in sessions.iter() {
                    session.send(message.clone(), protocols.clone()).unwrap();
                }
            }
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

        // todo! (eulerfrog) add handle to allow router to shut down and stop sending
        // requests
        tokio::spawn(Self::run(self.inner.clone(), sessions, protocols));

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // all messages at this point should be from udp port 520
        // messages are either request or response

        // obtain the pci slot that the message was received from
        let demux_info = *control.get::<DemuxInfo>().ok_or(DemuxError::Other)?;
        let ipv4_header_info = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;
        
        let slot = demux_info.slot;
        let router_address = ipv4_header_info.source;

        let remote_endpoint = Endpoint::new(router_address, 512);
        let local_endpoint = Endpoint::new(self.local_ips[slot as usize], 512);
        let endpoints = Endpoints::new(local_endpoint, remote_endpoint);

        // parse packet from message
        let packet = match RipPacket::from_bytes(message.iter()) {
            Ok(packet) => packet,
            Err(_) => return Err(DemuxError::Header),
        };
        
        match packet.header.command {
            Operation::Request => {
                let udp = protocols.protocol::<Udp>().expect("Rip requires UDP to work").clone();
                let packets = self.inner.process_request(packet);
                
                for packet in packets.iter() {
                    let message = Message::new(RipPacket::build(packet));
                    let id = self.id();
                    let udp = udp.clone();
                    let protocols = protocols.clone();

                    // send response message back to router
                    tokio::spawn(async move {
                        let result = udp.open_and_listen(id, endpoints, protocols.clone()).await;
                        let session = match result {
                            Ok(session) => session,
                            Err(_) => return,
                        };
                        let _ = session.send(message, protocols);
                    });
                }
            }
            Operation::Response => {
                // update router table accordingly
                self.inner.process_response(router_address, slot, packet);
            }
        }

        Ok(())
    }
}
