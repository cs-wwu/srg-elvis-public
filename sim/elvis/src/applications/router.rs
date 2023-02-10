pub struct Router {
    outgoing: Arc<RwLock<Option<Vec<SharedSession>>>>,
    ip_table: IpToTapSlot,
    // arp_table: default!()
}

impl Router {
    pub fn new() {
        todo!();
    }

    pub fn new_shared() {
        todo!();
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
        let pci = protocols.protocol(Pci::Id).expect("No such protocol");

        // query the number of taps in our pci session
        let number_taps = pci.query(Pci::SLOT_COUNT_QUERY_KEY).unwrap();

        *self.outgoing.write().unwrap() = Vec::<SharedSession>::new(number_taps);

        // 
        for i in (0..number_taps) {
            *self.outgoing.write().unwrap().get(i).unwrap() = Some(pci.clone().open(
                Self::ID,
                participants.clone(),
                protocols.clone(),
            )?);
        }
        tokio::spawn(async move {
            initialized.wait().await;
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
        let address = Ipv4::get_remote_address(&context.control);

        // put destination address through ip table
        let destination = table.get(address);

        *self.outgoing
            .read()
            .unwrap()
            .as_ref()
            .get(destination)
            .unwrap()
            .clone()
            .send(message, context)?;

        Ok(())
    }
}