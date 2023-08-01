use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address},
        pci::DemuxInfo,
        Endpoint, Endpoints, Udp,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;

use crate::applications::ArpRouter;

use super::rip_parsing::{Operation, RipPacket};

// number of seconds between each update
const UPDATE: u64 = 1;
pub struct RipRouter {
    local_ips: Vec<Ipv4Address>,
}

impl RipRouter {
    pub fn new(
        // Maps subnet to a given router ip.
        // Setting route to none sets the destination ip to the destination
        // ip in the received packet so the router can send to a local network.
        local_ips: Vec<Ipv4Address>,
    ) -> Self {
        RipRouter {
            local_ips: local_ips.clone(),
        }
    }

    pub async fn run(
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
            let packets = protocols.protocol::<ArpRouter>().expect("RipRouter requires ArpRouter").generate_request();

            // broadcast request to all adjacent routes
            for packet in packets.iter() {
                let message = Message::new(packet.build());
                for session in sessions.iter() {
                    session.send(message.clone(), protocols.clone()).unwrap();
                }
            }
            println!("broadcasted.");
        }
    }
}

#[async_trait::async_trait]
impl Protocol for RipRouter {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        println!("on the way");

        let udp = protocols
            .clone()
            .protocol::<Udp>()
            .expect("RipRouter requires Udp");
        
        initialized.wait().await;
        
        let mut sessions = Vec::<Arc<dyn Session>>::new();

        let remote_endpoint = Endpoint::new(Ipv4Address::SUBNET, 520);
        let broadcast_endpoint = Endpoint::new(Ipv4Address::SUBNET, 520);

        udp.listen(self.id(), broadcast_endpoint, protocols.clone()).ok();

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
        
        
        // todo! (eulerfrog) add handle to allow router to shut down and stop sending
        // requests
        println!("got here");
        tokio::spawn(Self::run(sessions, protocols));

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        // println!("i been demuxed");
        // all messages at this point should be from udp port 520
        // messages are either request or response

        // obtain the pci slot that the message was received from
        let demux_info = *control.get::<DemuxInfo>().ok_or(DemuxError::Other)?;
        let ipv4_header_info = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;
        // let arp_router = protocols.protocol::<ArpRouter>().ok_or(DemuxError::Other)?;

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
                println!("received request");
                let udp = protocols
                    .protocol::<Udp>()
                    .expect("Rip requires UDP to work")
                    .clone();
                let packets = protocols.protocol::<ArpRouter>().expect("RipRouter requires ArpRouter").process_request(packet);
                println!("{}", packets.len());

                for packet in packets.iter() {
                    let message = Message::new(RipPacket::build(packet));
                    let id = self.id();
                    let udp = udp.clone();
                    let protocols = protocols.clone();

                    // send response message back to router
                    tokio::spawn(async move {
                        println!("sending request");
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
                println!("received response");
                protocols.protocol::<ArpRouter>().expect("RipRouter requires ArpRouter").process_response(router_address, slot, packet);
            }
        }

        Ok(())
    }
}
