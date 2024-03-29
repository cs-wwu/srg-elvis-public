use elvis_core::{
    machine::PciSlot,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address, ProtocolNumber},
        AddressPair, Arp, Ipv4, Pci,
    },
    Control, IpTable, Machine, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

#[derive(Debug, Clone, Eq, PartialEq)]
/// Static router that uses arp to route messages to the correct location
/// created by providing a table mapping subnet to router ip and pci slot
/// requires a local ip to be specified for each pci session
pub struct ArpRouter {
    ip_table: IpTable<(Option<Ipv4Address>, PciSlot)>,
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
            ip_table,
            local_ips,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for ArpRouter {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialize: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        let ipv4 = machine
            .protocol::<Ipv4>()
            .expect("Arp Router requires IPv4");

        let arp = machine.protocol::<Arp>().expect("Arp Router requires Arp");

        ipv4.listen(
            self.id(),
            Ipv4Address::CURRENT_NETWORK,
            machine.clone(),
            ProtocolNumber::TCP,
        )
        .unwrap();

        ipv4.listen(
            self.id(),
            Ipv4Address::CURRENT_NETWORK,
            machine,
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
        machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        let mut ipv4_header = *control.get::<Ipv4Header>().ok_or(DemuxError::Other)?;
        ipv4_header.time_to_live -= 1;
        if ipv4_header.time_to_live == 0 {
            return Ok(());
        }

        message.header(ipv4_header.serialize().or(Err(DemuxError::Other))?);

        let pair = self
            .ip_table
            .get_recipient(ipv4_header.destination)
            .ok_or(DemuxError::Other)?;

        let gateway = match pair.0 {
            Some(address) => address,
            // allows router to send packet back to local network
            None => ipv4_header.destination,
        };
        let slot = pair.1;

        let arp = machine.protocol::<Arp>().unwrap();

        let address_pair = AddressPair {
            local: self.local_ips[slot as usize],
            remote: gateway,
        };

        tokio::spawn(async move {
            let arp_result = arp.resolve(address_pair, slot, machine.clone()).await;
            match arp_result {
                Err(_) => {}
                Ok(mac) => {
                    let session = machine.protocol::<Pci>().unwrap().open(slot);
                    session
                        .send_pci(message, Some(mac), TypeId::of::<Ipv4>())
                        .expect("failed to send");
                }
            }
        });

        Ok(())
    }
}
