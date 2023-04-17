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
    // Receive a message
    let req = socket.recv(32).await.unwrap();
    println!(
        "SERVER: Request Received: {:?}",
        String::from_utf8(req).unwrap()
    );

    // Send a message
    let resp = "Major Tom to Ground Control";
    println!("SERVER: Sending Response: {:?}", resp);
    socket.send(resp).unwrap();

    // Receive a message (Also example usage of recv_msg)
    let _ack = socket.recv_msg().await.unwrap();
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
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = self.sockets.clone();
        let local_port = self.local_port;

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let listen_socket = sockets
                .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols).await
                .unwrap();

            // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
            let local_sock_addr = SocketAddress::new_v4(Ipv4Address::CURRENT_NETWORK, local_port);
            listen_socket.bind(local_sock_addr).unwrap();

            // Listen for incoming connections, with a maximum backlog of 10
            listen_socket.listen(10).unwrap();
            println!("SERVER: Listening for incoming connections");

            // Wait on ititialization before sending or receiving any message from the network
            initialized.wait().await;

            let mut tasks = Vec::new();
            // Continuously accept incoming connections in a loop, spawning a
            // new tokio task to handle each accepted connection
            loop {
                // Accept an incoming connection
                let socket = listen_socket.accept().await.unwrap();
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

            // Shut down the simulation
            println!("SERVER: Shutting down");
            shutdown.shut_down();
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
