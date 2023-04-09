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

pub struct SocketServer {
    /// The Sockets API
    sockets: Arc<Sockets>,
    /// The port to capture a message on
    local_port: u16,
}

impl SocketServer {
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
    // Send a connection response
    println!("SERVER: Sending connection response");
    socket.clone().send("ACK").unwrap();

    // Receive a message
    let req = socket.clone().recv(32).await.unwrap();
    println!(
        "SERVER: Request Received: {:?}",
        String::from_utf8(req).unwrap()
    );

    // Send a message
    let resp = "Major Tom to Ground Control";
    println!("SERVER: Sending Response: {:?}", resp);
    socket.clone().send(resp).unwrap();

    // Receive an ackowledgement (Example usage of recv_msg)
    let _ack = socket.clone().recv_msg().await.unwrap();
    println!("SERVER: Ackowledgement Received");
}

impl Application for SocketServer {
    const ID: Id = Id::from_string("Socket Server");

    fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // Create a new IPv4 Datagram Socket
        let sockets = self.sockets.clone();
        let local_port = self.local_port;
        // let text = self.text;
        // let message = self.message.clone();

        tokio::spawn(async move {
            // Wait on ititialization before receiving any message from the network
            initialized.wait().await;
            let listen_socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
                .unwrap();

            // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
            let local_sock_addr = SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, local_port);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections
            listen_socket.clone().listen(0).unwrap();
            println!("SERVER: Listening for incoming connections");

            let mut tasks = Vec::new();
            loop {
                // Accept an incoming connection
                let socket = listen_socket.clone().accept().await.unwrap();
                println!("SERVER: Connection accepted");

                // Spawn a new tokio task for handling communication
                // with the new client
                tasks.push(tokio::spawn(async move {
                    communicate_with_client(socket).await;
                }));

                // This particular example server tracks the number of clients
                // served, stops accepting new connections after the third,
                // and shuts down the simulation once communication with
                // the third has ended
                if tasks.len() >= 3 {
                    while !tasks.is_empty() {
                        tasks.pop().unwrap().await.unwrap()
                    }
                    break;
                }
            }
            println!("SERVER: Shutting down");
            shutdown.shut_down();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
