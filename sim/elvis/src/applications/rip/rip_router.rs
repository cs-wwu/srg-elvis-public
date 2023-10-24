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
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;

use crate::applications::ArpRouter;

use super::rip_parsing::{Operation, RipPacket};

// number of seconds between each update
const UPDATE_INTERVAL: u64 = 1;
pub struct RipRouter {
    local_ips: Vec<Ipv4Address>,
    name: Option<String>,
}

impl RipRouter {
    pub fn new(
        local_ips: Vec<Ipv4Address>,
    ) -> Self {
        RipRouter {
            local_ips: local_ips.clone(),
            name: None,
        }
    }

    pub fn debug(mut self, name: String) -> Self {
        let mut rng = rand::thread_rng();
        let name = rng.gen_range(0..1000).to_string();
        self.name = Some(name);
        println!("Mah name is {:?}", self.name);
        self
    }

    pub async fn run(sessions: Vec<Arc<dyn Session>>, protocols: ProtocolMap) {
        // send initial full table request for each udp session
        let ftr = Message::new(RipPacket::new_full_table_request().build());

        // possible bug: if a router does not receive this request then there is a chance that
        // its route is never discovered.
        for session in sessions.iter() {
            session.send(ftr.clone(), protocols.clone()).unwrap();
        }

        // every UPDATE seconds send a broadcast update to each of the
        // routers udp sessions to update the routers table
        loop {
            tokio::time::sleep(Duration::from_secs(UPDATE_INTERVAL)).await;

            // todo! (eulerfrog) figure out how to not send full table requests every
            // time a router wants an update
            for session in sessions.iter() {
                session.send(ftr.clone(), protocols.clone()).unwrap();
            }
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
        let udp = protocols
            .clone()
            .protocol::<Udp>()
            .expect("RipRouter requires Udp");

        initialized.wait().await;

        let mut sessions = Vec::<Arc<dyn Session>>::new();

        let broadcast_endpoint = Endpoint::new(Ipv4Address::SUBNET, 520);

        udp.listen(self.id(), broadcast_endpoint, protocols.clone())
            .ok();

        for ip in self.local_ips.iter() {
            let local_endpoint = Endpoint::new(*ip, 520);
            let session = match udp
                .open_and_listen(
                    self.id(),
                    Endpoints {
                        local: local_endpoint,
                        remote: broadcast_endpoint.clone(),
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
        // all messages at this point should be from udp port 520
        // messages are either request or response
        
        // obtain the pci slot that the message was received from
        let demux_info = *control.get::<DemuxInfo>().ok_or(DemuxError::Other)?;
        let ipv4_header_info = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;

        let slot = demux_info.slot;
        let router_address = ipv4_header_info.source;
        // discard packets coming from this router
        
        let packet = match RipPacket::from_bytes(message.iter()) {
            Ok(packet) => packet,
            Err(_) => return Err(DemuxError::Header),
        };

        

        let remote_endpoint = Endpoint::new(router_address, 520);
        let local_endpoint = Endpoint::new(self.local_ips[slot as usize], 520);
        let endpoints = Endpoints::new(local_endpoint, remote_endpoint);
        

        // parse packet from message
        if self.local_ips[slot as usize] == router_address {
            // if packet.header.command == Operation::Response {
            //     println!("Discarded a response packet {:?} from {:?}", endpoints.remote, endpoints.local);
            // }
            return Ok(());
        }
        print!("--------------------------------------------------------------\n");
        if let Some(name) = self.name.clone(){
            println!("Router local ips {:?}", self.local_ips);
            println!("Router {} local: {:?}, remote {:?}", name, local_endpoint, remote_endpoint);
        }
        match packet.header.command {
            Operation::Request => {
                
                let udp = protocols
                    .protocol::<Udp>()
                    .expect("Rip requires UDP to work")
                    .clone();

                let packets = protocols
                    .protocol::<ArpRouter>()
                    .expect("RipRouter requires ArpRouter")
                    .process_request(router_address, packet);

                // if let Some(name) = self.name.clone(){
                //     println!("Router {} recieved request, packet is {:?}", name, packets);
                // }

                for packet in packets.iter() {
                    let response_message = Message::new(RipPacket::build(packet));
                    let id = self.id();
                    let udp = udp.clone();
                    let protocols = protocols.clone();
                    // println!("Response sent to before move {:?} from {:?}", endpoints.remote, endpoints.local);
                    tokio::spawn(async move {
                        let result = udp.open_for_sending(id, endpoints, protocols.clone()).await;
                        let session = match result {
                            Ok(session) => session,
                            Err(_) => {
                                println!{"Error ocurred in open udp"};
                                return},
                        };
                        let _ = session.send(response_message, protocols);
                    });
                    // println!("Response sent to {:?} from {:?}", endpoints.remote, endpoints.local);
                }
                // if let Some(name) = self.name.clone(){
                //     println!("Router {} recieved request, sent to {:?}", name, endpoints.remote);
                // }
            }
            Operation::Response => {
                // update router table accordingly
                if let Some(name) = self.name.clone(){
                    println!("Router {} recieved response info from {}", name, router_address);
                }
                protocols
                    .protocol::<ArpRouter>()
                    .expect("RipRouter requires ArpRouter")
                    .process_response(router_address, slot, packet);
            }
        }

        Ok(())
    }
}
