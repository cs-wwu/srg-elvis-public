use elvis_core::{
    message::Message,
    network::Mac,
    protocol::Context,
    protocols::{
        ipv4::{Ipv4Address, IpToTapSlot},
        udp::Udp,
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

    pub fn new_shared(ip_table: IpToTapSlot, arp_table: Arp) {
        
    }
}

impl Application for Router {
    /// A unique identifier for the application used by controls and the protocol map
    const ID: Id = Id::from_string("Router");

    /// Gives the application an opportunity to set up before the simulation
    /// begins.
    fn start(
        self: Arc<Self>,
        shutdown: Sender<()>,
        initialize: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // create a control. This control needs to be filled with some information
        let mut participants = Control::new();
        
        // get the pci protocol
        let pci = protocols.protocol(Pci::ID).expect("No such protocol").clone();

        // query the number of taps in our pci session
        let number_taps = pci.query(Pci::SLOT_COUNT_QUERY_KEY).unwrap().to_u32().unwrap();

        let mut sessions = Vec::with_capacity(number_taps as usize);

        for _i in 0..number_taps {
            let val = pci.clone().open(
                Self::ID,
                participants.clone(),
                protocols.clone(),
            )
            .unwrap();

        }

        *self.outgoing.write().unwrap() = Some(sessions);

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
        context: Context
    ) -> Result<(), ApplicationError> {
        // obtain destination address of the message
        let address = Ipv4::get_remote_address(&context.control).unwrap();

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
            .send(message, context);

        Ok(())
    }
}