use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        sockets::{
            socket::{ProtocolFamily, SocketAddress, SocketType},
            Sockets,
        },
        user_process::{Application, ApplicationError, UserProcess},
    },
    Id, ProtocolMap,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Barrier};

pub struct SocketSendMessage {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The text of the message to send
    text: &'static str,
    /// The IP address to send from
    local_ip: Ipv4Address,
    /// The port to send from
    local_port: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send on
    remote_port: u16,
}

impl SocketSendMessage {
    pub fn new(
        sockets: Arc<Sockets>,
        text: &'static str,
        local_ip: Ipv4Address,
        local_port: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Self {
        Self {
            sockets,
            text,
            local_ip,
            local_port,
            remote_ip,
            remote_port,
        }
    }

    pub fn new_shared(
        sockets: Arc<Sockets>,
        text: &'static str,
        local_ip: Ipv4Address,
        local_port: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
    ) -> Arc<UserProcess<Self>> {
        UserProcess::new_shared(Self::new(
            sockets,
            text,
            local_ip,
            local_port,
            remote_ip,
            remote_port,
        ))
    }
}

impl Application for SocketSendMessage {
    const ID: Id = Id::from_string("Socket Send");

    fn start(
        self: Arc<Self>,
        _shutdown: Sender<()>,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        tokio::spawn(async move {
            initialized.wait().await;

            // Create a new IPv4 Datagram Socket
            let socket = self
                .sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();

            // Bind the socket to your local address
            let local_sock_addr = SocketAddress::new_v4(self.local_ip, self.local_port);
            socket.clone().bind(local_sock_addr).unwrap();

            // "Connect" the socket to a remote address
            let remote_sock_addr = SocketAddress::new_v4(self.remote_ip, self.remote_port);
            socket.clone().connect(remote_sock_addr).unwrap();

            // Send a message
            println!("Sending Request: {:?}", self.text);
            socket.clone().send(self.text).unwrap();

            // Receive a message
            let msg = socket.clone().recv(32).await.unwrap();
            println!("Response Received: {:?}", String::from_utf8(msg));

            // Send another message
            println!("Sending Request: Shutdown");
            socket.clone().send("Shutdown").unwrap();
        });
        Ok(())
    }

    fn receive(
        self: Arc<Self>,
        _message: Message,
        _context: Context,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}
