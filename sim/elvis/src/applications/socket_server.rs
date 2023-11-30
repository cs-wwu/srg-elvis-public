use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, Socket, SocketType},
        Endpoint, SocketAPI,
    },
    Control, Machine, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc};
use tokio::{sync::Barrier, task::JoinSet};

#[derive(Clone)]
pub struct SocketServer {
    /// The port to capture a message on
    local_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
    /// The number of clients to accept
    num_clients: usize,
    /// Whether to output text or not
    output: bool,
}

impl SocketServer {
    pub fn new() -> Self {
        Self {
            local_port: 0xbeef,
            transport: SocketType::Stream,
            num_clients: 1,
            output: false,
        }
    }

    pub fn local_port(mut self, local_port: u16) -> Self {
        self.local_port = local_port;
        self
    }

    pub fn transport(mut self, transport: SocketType) -> Self {
        self.transport = transport;
        self
    }

    pub fn num_clients(mut self, num_clients: usize) -> Self {
        self.num_clients = num_clients;
        self
    }

    pub fn output(mut self, output: bool) -> Self {
        self.output = output;
        self
    }
}

async fn communicate_with_client(mut socket: Socket, server_num: u16, output: bool) {
    if output {
        println!("SERVER ({:?}): Waiting for request...", server_num);
    }

    // Receive a message (In two pieces to test recv() functionality)
    match socket.recv(20).await {
        Ok(req) => {
            if output {
                print!(
                    "SERVER ({:?}): Request Received: \"{}",
                    server_num,
                    String::from_utf8(req.to_vec()).unwrap()
                )
            };
        }
        Err(e) => {
            println!("SERVER ({:?}) Error: {:?}", server_num, e)
        }
    }
    match socket.recv(20).await {
        Ok(req) => {
            if output {
                println!("{}\"", String::from_utf8(req.to_vec()).unwrap())
            };
        }
        Err(e) => {
            println!("SERVER ({:?}) Error: {:?}", server_num, e)
        }
    }

    // Send a message
    let resp = format!("({}) Major Tom to Ground Control", server_num);
    if output {
        println!("SERVER ({:?}): Sending Response: {:?}", server_num, resp);
    }
    socket.send(resp).unwrap();

    // Receive a message (Also example usage of recv_msg)
    // println!("SERVER: Waiting for acknowledgement...");
    match socket.recv_msg().await {
        Ok(ack) => {
            if output {
                println!(
                    "SERVER ({:?}): Acknowledgement Received: {:?}",
                    server_num,
                    String::from_utf8(ack.to_vec()).unwrap()
                )
            };
        }
        Err(e) => {
            println!("SERVER ({:?}) Error: {:?}", server_num, e)
        }
    }

    socket.close();
}

#[async_trait::async_trait]
impl Protocol for SocketServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: DoneSender,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = machine
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;
        let local_port = self.local_port;
        let transport = self.transport;
        let num_clients = self.num_clients;
        let output = self.output;

        let mut listen_socket = sockets
            .new_socket(ProtocolFamily::INET, transport, machine.clone())
            .await
            .unwrap();

        // Bind the socket to Ipv4 [0.0.0.0] (Any Address) for listening
        let local_sock_addr = Endpoint::new(Ipv4Address::CURRENT_NETWORK, local_port);
        listen_socket.bind(local_sock_addr).unwrap();

        // Listen for incoming connections, with a maximum backlog of 10
        listen_socket.listen(num_clients).unwrap();
        if self.output {
            println!("\nSERVER: Listening for incoming connections");
        }

        // Wait on ititialization before sending or receiving any message from the network
        initialized.wait().await;

        // Error checking, these calls *should* return errors.
        if listen_socket.listen(num_clients).is_ok() {
            return Err(StartError::Other);
        }
        if listen_socket.connect(local_sock_addr).await.is_ok() {
            return Err(StartError::Other);
        }

        // Error checking, a second socket should not be able to listen on the same port
        let mut listen_socket_2 = sockets
            .new_socket(ProtocolFamily::INET, transport, machine)
            .await
            .unwrap();
        listen_socket_2.bind(local_sock_addr).unwrap();
        if listen_socket_2.listen(num_clients).is_ok() {
            return Err(StartError::Other);
        }

        let mut tasks = JoinSet::new();
        let mut client_num = 1;
        // Continuously accept incoming connections in a loop, spawning a
        // new tokio task to handle each accepted connection
        loop {
            // Accept an incoming connection
            let socket = listen_socket.accept().await.unwrap();
            if self.output {
                println!("SERVER: Connection {:?} accepted", client_num);
            }

            // Spawn a new tokio task for handling communication
            // with the new client
            tasks.spawn(async move {
                communicate_with_client(socket, client_num, output).await;
            });

            client_num += 1;

            // This particular example server tracks the number of clients
            // served, stops accepting new connections after the third,
            // and shuts down the simulation once communication with
            // the third has ended
            if tasks.len() >= num_clients {
                while !tasks.is_empty() {
                    match tasks.join_next().await.unwrap() {
                        Ok(_) => {
                            if self.output {
                                println!("Remaining Clients: {:?}", tasks.len());
                            }
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                break;
            }
        }

        // Shut down the simulation
        if self.output {
            println!("SERVER: Shutting down");
        }
        listen_socket.close();
        shutdown.shut_down();
        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

impl Default for SocketServer {
    fn default() -> Self {
        Self::new()
    }
}
