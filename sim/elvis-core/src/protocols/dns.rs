//! An implementation of the Domain Name Structure

/// Serves as a tool for looking up the ['Ipv4Address'] of a host using its
/// known machine name (domain), and as the storage for an individual machine's
/// name to IP mappings.
pub struct Dns {
    /// A unique identifier for the protocol.
    pub const ID: Id = Id::from_string("DNS"),
    listen_bindings: DashMap<Ipv4Address, Id>,
    sessions: DashMap<SessionId, Arc<DnsSession>>,
    /// Mapping of names to IPs that is unique to each machine. When a machine
    /// connects to a host using DNS, the mapping is saved in the connecting
    /// machines DNS protocol.
    pub mut name_to_ip: HashMap<&str, Ipv4Address>,
}

impl Dns {
    /// Creates a new instance of the protocol.
    pub fn new() -> Self {
        Self {
            name_to_ip: HashMap<&str, Ipv4Address>::new();
            listen_bindings: Default::default(),
            sessions: Default::default(),
        }
    }

    /// Adds a new mapping to the name_to_ip cache.
    pub fn add_mapping() -> {

    }

    /// 
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