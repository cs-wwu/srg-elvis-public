use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, SocketType},
        Endpoint, SocketAPI,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

pub struct FakeDnsUser {
    /// Numerical ID
    client_id: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
}

impl FakeDnsUser {
    pub fn new(
        client_id: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
        transport: SocketType,
    ) -> Self {
        Self {
            client_id,
            remote_ip,
            remote_port,
            transport,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for FakeDnsUser {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        drop(_shutdown);

        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = protocols
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;

        let socket = sockets
            .new_socket(ProtocolFamily::INET, self.transport, protocols)
            .await
            .unwrap();

        // Wait on initialization before sending any message across the network
        initialized.wait().await;

        // "Connect" the socket to a remote address
        socket.connect_by_name("testserver.com".to_string(), self.remote_port).await;
        println!("CLIENT {}: Connected", self.client_id);

        // Send a message
        let req = "Ground Control to Major Tom";
        println!("CLIENT {}: Sending Request: {:?}", self.client_id, req);
        socket.send(req).unwrap();

        // Receive a message
        let resp = socket.recv(32).await.unwrap();
        println!(
            "CLIENT {}: Response Received: {:?}",
            self.client_id,
            String::from_utf8(resp).unwrap()
        );

        // Send a message
        println!("CLIENT {}: Sending Ackowledgement", self.client_id);
        socket.send("Ackowledged").unwrap();
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
