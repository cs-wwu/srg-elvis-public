use std::sync::Arc;

use crate::{
    shutdown::Shutdown,
    machine::{
        ProtocolMap,
        Machine,
    },
    protocols::{
        ipv4::Ipv4Address,
        sockets::socket::{ProtocolFamily, Socket, SocketError, SocketType},
        Sockets,
        Dns,
        dns::DnsType,
    },
};
use tokio::sync::Barrier;

pub struct NetworkAPI {
    socket_api: Arc<Sockets>,
    // TODO(giddinl2): add DNS field
    dns: Arc<Dns>,
}

impl NetworkAPI {
    pub fn new(local_ip: Option<Ipv4Address>) -> Self {
        Self {
            socket_api: Sockets::new(local_ip).shared(),
            dns: Dns::new(DnsType::CLI, Ipv4Address::DNS_AUTH).shared(),
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
    /// Finds the IP associated with the given domain name.
    fn get_host_by_name(
        &self,
        name: String,
    ) -> Result<Ipv4Address, SocketError> {
        // Get DNS protocol from this socket protocol's machine
        // let dns: Dns =  match protocols.protocol(Dns::ID) {
        //     Some(p) => p,
        //     None => {
        //         return Err(SocketError::Other);
        //     }
        // };

        match self.dns.get_mapping(name) {
            // Cache hit
            Ok(ip) => Ok(ip),

            // Cache miss
            Err(DnsError) => {
                // TODO(zachd9757): Check authoritative server
                Err(SocketError::Other)
            },
        }
    }

    // TODO(giddinl2): function to create a tcp_stream instance

    // TODO(giddinl2): function to create a tcp_listener instance
}

#[cfg(test)]
mod tests {

    use crate::new_machine;

    use super::*;

    #[tokio::test]
    /// Test for Sockets:get_host_by_name() when Dns cache is empty
    async fn ghbn_cache_miss() {
        // let sockets = Sockets::new(None).shared();
        let network_api = NetworkAPI::new(Some(Ipv4Address::CURRENT_NETWORK)).shared();

        let machine: Machine = new_machine![
            Dns::new(DnsType::CLI, Ipv4Address::CURRENT_NETWORK)
        ];

        let shutdown = Shutdown::new();
        let total_protocols: usize = machine.protocol_count();
        let initialized = Arc::new(Barrier::new(total_protocols));
        let protocols: ProtocolMap = machine.protocols.clone();
        
        machine.start(shutdown.clone(), initialized.clone());
        
        let ip: Result<Ipv4Address, SocketError> =
            network_api.get_host_by_name("DNE".to_string());

        assert_eq!(ip, Err(SocketError::Other));

        // TODO(zachd9757) potentially expand this test to establish a
        // machine-to-machine connection and check if the cache is auto-updated



    }
}
