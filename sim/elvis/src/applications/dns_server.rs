
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        Endpoint,
        Udp,
        Dns,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        SocketAPI,
    },
    Control, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;


pub const PORT_NUM: u16 = 53;

pub struct DnsServer {
    /// The Sockets API
    sockets: Arc<Socket>,
    /// The port to capture a message on
    local_port: u16,
}

impl DnsServer {
    pub fn new(
            sockets: Arc<Socket>,
            local_port: u16,
            remote_ip: Ipv4Address,
        ) -> Self {
        Self {
            sockets,
            local_port,
        }
    }

    // pub fn shared(self) -> Arc<UserProcess<Self>> {
    //     UserProcess::new(self).shared()
    // }
}

#[async_trait::async_trait]
impl Protocol for DnsServer {
    // const ID: Id = Id::from_string("DNS Server");

    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let udp = protocols.protocol::<Udp>().unwrap();
        udp.listen(
            self.id(),
            Endpoint::new(Ipv4Address::DNS_AUTH, 53), protocols
        ).unwrap();
        initialized.wait().await;
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

// impl Default for DnsServer {
//     fn default() -> Self {
//         Self {
//             sockets: Sockets::new(Some(BROADCAST)).shared(),
//         }
//     }
// }
