use elvis_core::{
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::ipv4::ipv4_parsing::Ipv4Header,
    protocols::{
        ipv4::{IpToTapSlot, Ipv4Address},
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Pci,
    },
    session::SharedSession,
    Control, Id, Network, ProtocolMap,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::{mpsc::Sender, Barrier};

pub type Arp = HashMap<Ipv4Address, Mac>;

pub struct Router {
    outgoing: RwLock<Vec<SharedSession>>,
    ip_table: IpToTapSlot,
    arp_table: Arp,
}

impl Router {
    pub fn new(ip_table: IpToTapSlot, arp_table: Arp) -> Self {
        Self {
            outgoing: Default::default(),
            ip_table,
            arp_table,
        }
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

impl Application for Router {
    /// A unique identifier for the application used by controls and the protocol map
    const ID: Id = Ipv4::ID;

    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    fn start(
        &self,
        _shutdown: Sender<()>,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // get the pci protocol
        let pci = protocols.protocol(Pci::ID).expect("No such protocol");

        // query the number of taps in our pci session
        let number_taps = pci
            .clone()
            .query(Pci::SLOT_COUNT_QUERY_KEY)
            .expect("could not get slot count")
            .to_u64()
            .expect("could not unwrap u32");

        let mut sessions = Vec::with_capacity(number_taps as usize);

        for i in 0..number_taps {
            let mut participants = Control::new();
            Pci::set_pci_slot(i as u32, &mut participants);
            let val = pci
                .clone()
                .open(Self::ID, participants.clone(), protocols.clone())
                .expect("could not open session");
            sessions.push(val);
        }

        *self
            .outgoing
            .write()
            .expect("could not put array in outgoing") = sessions;

        tokio::spawn(async move {
            initialize.wait().await;
        });
        Ok(())
    }

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn receive(&self, message: Message, mut context: Context) -> Result<(), ApplicationError> {
        // obtain destination address of the message
        // cant use this as we dont have an ipv4 protocol in the router
        // should probably extract it from the message object somehow

        // if the header cant parse drop the packet
        let header: Ipv4Header =
            Ipv4Header::from_bytes(message.iter()).or(Err(ApplicationError::Other))?;

        let address = header.destination;

        if let Some(destination_mac) = self.arp_table.get(&address) {
            Network::set_destination(*destination_mac, &mut context.control);
        }

        Network::set_protocol(Ipv4::ID, &mut context.control);

        // put destination address through ip table
        let destination = match self.ip_table.get(&address) {
            Some(destination) => *destination,
            None => return Ok(()),
        };

        self.outgoing
            .read()
            .expect("could not get outgoing as reference")
            .get(destination as usize)
            .expect("Could not send message")
            .clone()
            .send(message, context)?;

        Ok(())
    }
}
