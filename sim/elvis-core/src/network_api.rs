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
    // TODO(giddinl2): add DNS field
}

impl NetworkAPI {
    pub fn new(local_ip: Option<Ipv4Address>) -> Self {
        Self {
            socket_api: Sockets::new(local_ip).shared(),
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

    // TODO(giddinl2): function to return Arc of DNS

    // TODO(giddinl2): get_host_by_name function

    // TODO(giddinl2): function to create a tcp_stream instance

    // TODO(giddinl2): function to create a tcp_listener instance
}
