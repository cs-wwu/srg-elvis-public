//! An implementation of the Domain Name Structure

pub struct Dns {

}

impl Dns {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::from_string("DNS");

    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl Protocol for Dns {
    fn id(self: Arc<Self>) -> Id {
        Self::ID
    }

    fn open(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<SharedSession, OpenError> {
        //TODO
    }

    fn listen(
        self: Arc<Self>,
        upstream: Id,
        participants: Control,
        protocols: ProtocolMap,
    ) -> Result<(), ListenError> {
        //TODO
    }

    fn demux(
        self: Arc<Self>,
        message: Message,
        caller: SharedSession,
        context: Context,
    ) -> Result<(), DemuxError> {
        //TODO
    }

}