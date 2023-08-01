use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        socket_api::socket::{ProtocolFamily, SocketType},
        SocketAPI,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

pub struct DnsTestClient {
    /// The port to send to
    remote_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
}

impl DnsTestClient {
    pub fn new(remote_port: u16, transport: SocketType) -> Self {
        Self {
            remote_port,
            transport,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for DnsTestClient {
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
        // println!("CLIENT: Connecting to testserver.com");
        socket
            .connect_by_name("testserver.com".to_string(), self.remote_port)
            .await
            .unwrap();
        // println!("CLIENT: Connected");

        // Send a message
        let req = "Ground Control to Major Tom";
        // println!("CLIENT: Sending test Request: {:?}", req);
        socket.send(req).unwrap();

        // Receive a message
        let resp = socket.recv(32).await.unwrap();
        // println!(
        //     "CLIENT: Response Received: {:?}",
        //     String::from_utf8(resp).unwrap()
        // );

        // Send a message
        // println!("CLIENT: Sending Ackowledgement");
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
