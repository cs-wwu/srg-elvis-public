use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, SocketType},
        user_process::{Application, ApplicationError, UserProcess},
        Endpoint, SocketAPI,
    },
    Control, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

pub struct SocketClient {
    /// Numerical ID
    client_id: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
}

impl SocketClient {
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

    pub fn process(self) -> UserProcess<Self> {
        UserProcess::new(self)
    }
}

impl Application for SocketClient {
    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = protocols
            .protocol::<SocketAPI>()
            .ok_or(ApplicationError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let remote_ip = self.remote_ip;
        let remote_port = self.remote_port;
        let client_id = self.client_id;
        let transport = self.transport;

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let socket = sockets
                .new_socket(ProtocolFamily::INET, transport, protocols)
                .await
                .unwrap();

            // Wait on initialization before sending any message across the network
            initialized.wait().await;

            // "Connect" the socket to a remote address
            let remote_sock_addr = Endpoint::new(remote_ip, remote_port);
            println!("CLIENT {}: Attempting to connect...", client_id);
            socket.connect(remote_sock_addr).await.unwrap();
            println!("CLIENT {}: Connected", client_id);

            // Send a message
            let req = "Ground Control to Major Tom";
            println!("CLIENT {}: Sending Request: {:?}", client_id, req);
            socket.send(req).unwrap();

            // Receive a message
            // println!("CLIENT {}: Waiting for response...", client_id);
            let resp = socket.recv(32).await.unwrap();
            println!(
                "CLIENT {}: Response Received: {:?}",
                client_id,
                String::from_utf8(resp).unwrap()
            );

            // Send a message
            println!("CLIENT {}: Sending Ackowledgement", client_id);
            socket.send("Ackowledged").unwrap();
        });
        Ok(())
    }

    fn receive(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
