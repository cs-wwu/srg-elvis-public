use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        Endpoint, SocketAPI,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::sync::Barrier;

#[derive(Clone)]
pub struct SocketServer {
    /// The port to capture a message on
    local_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
}

impl SocketServer {
    pub fn new(local_port: u16, transport: SocketType) -> Self {
        Self {
            local_port,
            transport,
        }
    }
}

async fn communicate_with_client(socket: Arc<Socket>) {
    // Receive a message
    println!("SERVER: Waiting for request...");
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
    // println!("SERVER: Waiting for awkowledgement...");
    let _ack = socket.recv_msg().await.unwrap();
    println!("SERVER: Ackowledgement Received");
}

#[async_trait::async_trait]
impl Protocol for SocketServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = protocols
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = self.local_port;
        let transport = self.transport;

        let listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, protocols)
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::CURRENT_NETWORK, local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(10).unwrap();
        println!("\nSERVER: Listening for incoming connections");

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
