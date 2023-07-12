use elvis_core::{
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        arp::subnetting::Ipv4Mask,
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address},
        AddressPair, Arp, Ipv4, Pci,
    },
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use std::{cmp::min, sync::RwLock};
use tokio::sync::Barrier;

use super::rip_parsing::{RipEntry, RipPacket};

// entry representing next hop, outgoing interface, metric and route change flag
pub type Rte = (Ipv4Address, PciSlot, u32, bool);
const INFINITY: u32 = 16;

#[derive(Debug)]
pub struct ArpRouter {
    ip_table: RwLock<IpTable<Rte>>,
    local_ip: Ipv4Address,
}

impl ArpRouter {
    pub fn new(ip_table: IpTable<(Ipv4Address, PciSlot)>, local_ip: Ipv4Address) -> Self {
        Self {
            ip_table: RwLock::new(ip_table.into()),
            local_ip,
        }
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
            let mut count: u32 = 0;

            for entry in self.ip_table.read().unwrap().iter() {
                count += 1;

                let element: RipEntry =
                    RipEntry::new_entry(entry.0 .0, entry.1 .0, entry.0 .1, entry.1 .2);

                frame.push(element);

                // every 25th entry add the current frame to the output vector
                if count % 25 == 0 {
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
                entry.metric = route.2;
            } else {
                entry.metric = INFINITY;
            }
        }

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
            // + the cost of traveling to 
            let metric = min(entry.metric + 1, INFINITY);

            let destination = entry.ip_address;
            let mask = entry.subnet_mask;

            match ip_table_ref.get_recipient(destination) {
                Some(recipient) => {
                    if recipient.2 > metric {
                        ip_table_ref.add(
                            (recipient.0, Ipv4Mask::from_bitcount(recipient.1)),
                            (neighbor_ip, neighbor_slot, metric, true),
                        )
                    }
                }
                None => {
                    if metric < INFINITY {
                        ip_table_ref.add(
                            (destination, mask),
                            (neighbor_ip, neighbor_slot, metric, true),
                        )
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

        ipv4.listen(self.id(), Ipv4Address::CURRENT_NETWORK, protocols)
            .unwrap();

        arp.listen(self.local_ip);

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

        // TODO(hardint): Fragmentation
        message.header(ipv4_header.serialize().or(Err(DemuxError::Other))?);

        let pair = self
            .ip_table
            .read()
            .unwrap()
            .get_recipient(ipv4_header.destination)
            .ok_or(DemuxError::Other)?;

        let gateway = pair.0;
        let slot = pair.1;

        let arp = protocols.protocol::<Arp>().unwrap();

        let address_pair = AddressPair {
            local: self.local_ip,
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
