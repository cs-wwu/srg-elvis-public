use std::sync::Arc;

use crate::{
    protocols::{
        ipv4::Ipv4Address,
        sockets::socket::{ProtocolFamily, Socket, SocketError, SocketType},
        Sockets,
    },
    ProtocolMap,
};

pub struct NetworkAPI {
    socket_api: Arc<Sockets>,
    // TODO: dns: Arc<DNS>,
}

impl NetworkAPI {
    pub fn new(local_ip: Option<Ipv4Address>) -> Self {
        Self {
            socket_api: Sockets::new(local_ip).shared(),
            // TODO: dns: DNS::new().shared()
        }
    }

    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn socket_api(&self) -> Arc<Sockets> {
        self.socket_api.clone()
    }

    pub async fn new_socket(
        &self,
        domain: ProtocolFamily,
        sock_type: SocketType,
        protocols: ProtocolMap,
    ) -> Result<Arc<Socket>, SocketError> {
        self.socket_api
            .new_socket(domain, sock_type, protocols)
            .await
    }

    // pub fn dns(&self) -> Arc<DNS> {
    //     self.dns.clone()
    // }
    
    // pub fn get_host_by_name(&self) -> Result<Ipv4Address, DNSError> {
    //     self.dns.get_host_by_name(self.socket_api.clone())
    // }

    // pub fn tcp_stream(&self, ip: Ipv4Address) -> Arc<TCPStream> {
    //     // TODO: create TCPStream
    // }

    // pub fn tcp_listener(&self, ip: Ipv4Address) -> Arc<TCPListener> {
    //     // TODO: create TCPListener
    // }

}
