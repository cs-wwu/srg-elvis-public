use elvis_core::{
    ip_table::Rte,
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        arp::subnetting::Ipv4Net,
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, ProtocolNumber},
        AddressPair, Arp, Ipv4, Pci,
    },
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use std::{cmp::min, sync::RwLock};
use tokio::sync::Barrier;

use super::rip_parsing::{RipEntry, RipPacket};

// entry representing next hop, outgoing interface, metric and route change flag
const INFINITY: u32 = 16;

#[derive(Debug)]
/// Static router that uses arp to route messages to the correct location
/// created by providing a table mapping subnet to router ip and pci slot
/// requires a local ip to be specified for each pci session
pub struct ArpRouter {
    ip_table: RwLock<IpTable<Rte>>,
    local_ips: Vec<Ipv4Address>,
}

impl ArpRouter {
    pub fn new(
        // Maps subnet to a given router ip.
        // Setting route to none sets the destination ip to the destination
        // ip in the received packet so the router can send to a local network.
        ip_table: IpTable<(Option<Ipv4Address>, PciSlot)>,
        local_ips: Vec<Ipv4Address>,
    ) -> Self {
        Self {
            ip_table: RwLock::new(ip_table.into()),
            local_ips,
        }
    }

    // generate rip packets for each entry in the router that has a
    // destination router to send to
    pub fn generate_request(&self) -> Vec<RipPacket> {
        let mut output: Vec<RipPacket> = Vec::new();
        let mut entries: Vec<RipEntry> = Vec::new();

        for entry in self.ip_table.read().unwrap().iter() {
            if let Some(next_hop) = entry.1.destination {
                let rip_entry =
                    RipEntry::new_entry(entry.0.id(), next_hop, entry.0.mask(), entry.1.metric);

                entries.push(rip_entry);
            }

            if entries.len() == 25 {
                output.push(RipPacket::new_request(entries));
                entries = Vec::new();
            }
        }

        output
    }

    // processes the packet of an incoming arp request and returns relevent
    // information to calling process
    pub fn process_request(&self, packet: RipPacket) -> Vec<RipPacket> {
        let mut output: Vec<RipPacket> = Vec::new();
        let mut entries = packet.entries;

        // if entries is 1 and metric and address family id of that entry is
        // 0, process whole table request
        if entries.len() == 1 && entries[0].address_family_id == 0 {
            let mut frame: Vec<RipEntry> = Vec::new();

            for entry in self.ip_table.read().unwrap().iter() {
                // is this correct?
                let next_hop = match entry.1.destination {
                    Some(addr) => addr,
                    None => Ipv4Address::from([0, 0, 0, 0]),
                };

                let element: RipEntry =
                    RipEntry::new_entry(entry.0.id(), next_hop, entry.0.mask(), entry.1.metric);

                frame.push(element);

                // every 25th entry add the current frame to the output vector
                if frame.len() == 25 {
                    output.push(RipPacket::new_response(frame));
                    frame = Vec::new();
                }
            }

            return output;
        }

        // otherwise obtain the metrics for each entry that exists on the routing table
        for mut entry in entries.iter_mut() {
            if let Some(route) = self
                .ip_table
                .read()
                .unwrap()
                .get_recipient(entry.ip_address)
            {
                entry.metric = route.metric;
            } else {
                entry.metric = INFINITY;
            }
        }

        println!("I got a request, my ip table is: {:#?}", self.ip_table);

        output.push(RipPacket::new_response(entries));
        output
    }

    pub fn process_response(
        &self,
        neighbor_ip: Ipv4Address,
        neighbor_slot: PciSlot,
        packet: RipPacket,
    ) {
        // processs response
        let entries = packet.entries;
        let mut ip_table_ref = self.ip_table.write().unwrap();

        for entry in entries {
            // cost of going to new route is metric provided by packet
            // + the cost of traveling to that destination
            let metric = min(entry.metric + 1, INFINITY);

            let destination = entry.ip_address;
            let mask = entry.subnet_mask;

            match ip_table_ref.get_recipient(destination) {
                Some(recipient) => {
                    if recipient.metric > metric {
                        ip_table_ref.add(
                            Ipv4Net::new(destination, mask),
                            Rte::new(Some(neighbor_ip), mask, neighbor_slot, metric),
                        );
                    }
                }
                None => {
                    if metric < INFINITY {
                        ip_table_ref.add(
                            Ipv4Net::new(destination, mask),
                            Rte::new(Some(neighbor_ip), mask, neighbor_slot, metric),
                        );
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Protocol for ArpRouter {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let ipv4 = protocols
            .protocol::<Ipv4>()
            .expect("Arp Router requires IPv4");

        let arp = protocols
            .protocol::<Arp>()
            .expect("Arp Router requires Arp");

        ipv4.listen(
            self.id(),
            Ipv4Address::CURRENT_NETWORK,
            protocols.clone(),
            ProtocolNumber::TCP,
        )
        .unwrap();

        ipv4.listen(
            self.id(),
            Ipv4Address::CURRENT_NETWORK,
            protocols,
            ProtocolNumber::UDP,
        )
        .unwrap();

        for ip in self.local_ips.iter() {
            arp.listen(*ip);
        }

        initialize.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        mut message: Message,
        _caller: Arc<dyn Session>,
        control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let mut ipv4_header = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;
        ipv4_header.time_to_live -= 1;
        if ipv4_header.time_to_live == 0 {
            return Ok(());
        }

        message.header(ipv4_header.serialize().or(Err(DemuxError::Other))?);

        let rte = self
            .ip_table
            .read()
            .unwrap()
            .get_recipient(ipv4_header.destination)
            .ok_or(DemuxError::Other)?;

        let gateway = match rte.destination {
            Some(address) => address,
            // allows router to send packet back to local network
            None => ipv4_header.destination,
        };

        let slot = rte.slot;

        let arp = protocols.protocol::<Arp>().unwrap();

        let address_pair = AddressPair {
            local: self.local_ips[slot as usize],
            remote: gateway,
        };

        tokio::spawn(async move {
            let arp_result = arp.resolve(address_pair, slot, protocols.clone()).await;
            match arp_result {
                Err(_) => {}
                Ok(mac) => {
                    let session = protocols.protocol::<Pci>().unwrap().open(slot);
                    session
                        .send_pci(message, Some(mac), TypeId::of::<Ipv4>())
                        .expect("failed to send");
                }
            }
        });

        Ok(())
    }
}
