use elvis_core::{
    machine::PciSlot,
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::{ipv4_parsing::Ipv4Header, Ipv4Address},
        AddressPair, Arp, Ipv4, Pci,
    },
    Control, IpTable, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;
use std::sync::RwLock;

#[derive(Debug)]
pub struct ArpRouter {
    ip_table: RwLock<IpTable<(Ipv4Address, PciSlot)>>,
    local_ip: Ipv4Address,
}

impl ArpRouter {
    pub fn new(ip_table: IpTable<(Ipv4Address, PciSlot)>, local_ip: Ipv4Address) -> Self {
        Self { ip_table: RwLock::new(ip_table), local_ip }
    }

    pub fn receive() {

    }

    pub fn send() {

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
