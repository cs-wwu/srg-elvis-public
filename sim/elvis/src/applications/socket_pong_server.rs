use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        ipv4::Ipv4Address,
        sockets::{
            socket::{ProtocolFamily, Socket, SocketAddress, SocketType},
            Sockets,
        },
        user_process::{Application, ApplicationError, UserProcess},
    },
    Id, ProtocolMap, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

pub struct SocketPongServer {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The port to capture a message on
    local_port: u16,
}

impl SocketPongServer {
    pub fn new(sockets: Arc<Sockets>, local_port: u16) -> Self {
        Self {
            sockets,
            local_port,
        }
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

async fn communicate_with_client(socket: Arc<Socket>) {
    loop {
        // Receive a message
        let mut ttl: u8 = *socket.clone().recv(8).await.unwrap().first().unwrap();

        // Send a message
        ttl -= 1;

        if ttl == 0 {
            break;
        } else {
            socket.clone().send(vec![ttl]).unwrap();
        }
    }
}

impl Application for SocketPongServer {
    const ID: Id = Id::from_string("Socket Pong Server");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = self.sockets.clone();
        let local_port = self.local_port;

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let listen_socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols)
                .await
                .unwrap();

            // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
            let local_sock_addr = SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, local_port);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections, with a maximum backlog of 10
            listen_socket.clone().listen(10).unwrap();

            // Wait on ititialization before sending or receiving any message from the network
            initialized.wait().await;
            
            // Accept an incoming connection
            let socket = listen_socket.clone().accept().await.unwrap();

            // Spawn a new tokio task for handling communication
            // with the new client
            communicate_with_client(socket).await;

            // Shut down the simulation
            shutdown.shut_down();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
