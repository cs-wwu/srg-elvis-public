use elvis_core::{
    protocols::ipv4::ipv4_parsing::Ipv4Header,
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::{
        ipv4::{Ipv4Address, IpToTapSlot},
        user_process::{Application, ApplicationError, UserProcess},
        Ipv4, Pci,
    },
    session::SharedSession,
    Control, Id, Network, ProtocolMap,
};
use std::{sync::{Arc, RwLock}, collections::HashMap};
use tokio::sync::{mpsc::Sender, Barrier};

pub type Arp = HashMap<Ipv4Address, Mac>;

pub struct Router {
    outgoing: Arc<RwLock<Option<Vec<SharedSession>>>>,
    ip_table: IpToTapSlot,
    arp_table: Arp
}

impl Router {
    pub fn new(ip_table: IpToTapSlot, arp_table: Arp) -> Self {
        Self {
            outgoing: Default::default(),
            ip_table: ip_table,
            arp_table: arp_table
        }
    }

    pub fn new_shared(ip_table: IpToTapSlot, arp_table: Arp) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(ip_table, arp_table))
    }
}

impl Application for Router {
    /// A unique identifier for the application used by controls and the protocol map
    const ID: Id = Ipv4::ID;

    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // get the pci protocol
        let pci = protocols.protocol(Pci::ID)
            .expect("No such protocol");

        // query the number of taps in our pci session
        let number_taps = pci.clone().query(Pci::SLOT_COUNT_QUERY_KEY)
            .expect("could not get slot count").to_u64()
            .expect("could not unwrap u32");

        let mut sessions = Vec::with_capacity(number_taps as usize);

        // println!("{}", number_taps);
        
        for i in 0..number_taps {
            println!("{}", i);
            let mut participants = Control::new();
            Pci::set_pci_slot(i as u32, &mut participants);
            let val = pci.clone().open(
                Self::ID,
                participants.clone(),
                protocols.clone(),
            )
            .expect("could not open session");
            sessions.push(val);
        }

        *self.outgoing.write().expect("could not put array in outgoing") = Some(sessions);

        tokio::spawn(async move {
            initialize.wait().await;
        });
        Ok(())
    }

    /// Called when the containing [`UserProcess`] receives a message over the
    /// network and gives the application time to handle it.
    fn receive(
        self: Arc<Self>, 
        message: Message, 
        mut context: Context
    ) -> Result<(), ApplicationError> {
        println!("yoooooooo");
        
        // obtain destination address of the message
        // cant use this as we dont have an ipv4 protocol in the router
        // should probably extract it from the message object somehow
        let header: Ipv4Header = Ipv4Header::from_bytes(message.iter()).expect("Could not parse message header");
        let address = header.source;

        if let Some(destination_mac) = self.arp_table.get(&address) {
            Network::set_destination(*destination_mac, &mut context.control);
        } 

        // put destination address through ip table
        let destination = self.ip_table.get(&address).unwrap().clone();

        self.clone().outgoing
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(destination as usize)
            .unwrap()
            .clone()
            .send(message, context)?;

        Ok(())
    }
}